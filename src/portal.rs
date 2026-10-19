//! Portal game rules, separate from generic Source asset/render modules.
use crate::assets::Assets;
/// Select game rules independently of platform-specific shell syntax.
/// An explicit explorer override remains available for diagnostics.
pub fn rules_enabled(bytes: &[u8], override_value: Option<&str>) -> Result<bool, String> {
    if let Some(value) = override_value {
        return match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err("SOURCE_PORTALS must be 0 or 1".into()),
        };
    }
    Ok(crate::entity::parse_entities(crate::lump(bytes, 0, 1)?)?
        .iter()
        .any(|entity| {
            matches!(
                entity.get("classname"),
                Some("prop_portal" | "weapon_portalgun")
            )
        }))
}

/// Procedural approximation of Portal's energy projectile, in Source units.
#[derive(Clone, Debug)]
pub struct Shot {
    pub channel: usize,
    pub muzzle: Vec3,
    pub target: Vec3,
    pub age: f32,
    pub duration: f32,
    rejection: Option<String>,
    eye: Vec3,
    direction: Vec3,
    facing: Vec3,
}
impl Shot {
    pub fn position(&self) -> Vec3 {
        self.muzzle
            .lerp(self.target, (self.age / self.duration).clamp(0., 1.))
    }
}
pub struct Game {
    pub shots: Vec<Shot>,
    impacts: Vec<(usize, Vec3, f32)>,
    pub automatic_surfaces: Option<Surfaces>,
    pub portals: [Option<Portal>; 2],
    names: [Option<String>; 2],
    initial_authored_fitted: bool,
    authored: std::collections::HashMap<String, (usize, Portal)>,
}
impl Game {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let mut game = Self {
            shots: Vec::new(),
            impacts: Vec::new(),
            automatic_surfaces: None,
            portals: [None; 2],
            names: [None, None],
            initial_authored_fitted: false,
            authored: std::collections::HashMap::new(),
        };
        for entity in crate::entity::parse_entities(crate::lump(bytes, 0, 1)?)? {
            if entity.get("classname") != Some("prop_portal") {
                continue;
            }
            let angles = crate::entity::vector(entity.get("angles").unwrap_or("0 0 0"))?;
            let rotation = crate::models::rotation(angles);
            let portal = Portal::new(entity.origin()?, rotation * Vec3::X, rotation * Vec3::Z);
            let index = usize::from(entity.get("PortalTwo") == Some("1"));
            let name = entity.get("targetname").unwrap_or("").to_owned();
            game.authored.insert(name.clone(), (index, portal));
            if entity.get("Activated") == Some("1") {
                game.portals[index] = Some(portal);
                game.names[index] = Some(name);
            }
        }
        Ok(game)
    }
    pub fn sync_runtime(
        &mut self,
        runtime: &mut crate::entity::Runtime,
        world: &mut crate::play::World,
    ) {
        if !self.initial_authored_fitted {
            if let Some(surfaces) = &self.automatic_surfaces {
                for portal in self.portals.iter_mut().flatten() {
                    if let Ok(fitted) = surfaces.posed(runtime).fit_authored(*portal) {
                        *portal = fitted;
                    }
                }
                self.initial_authored_fitted = true;
            }
        }
        for (index, eye, direction) in std::mem::take(&mut runtime.portal_shots) {
            runtime.sound_events.push(crate::audio::Event::one(
                runtime.sound_time(),
                if index == 0 {
                    "Weapon_Portalgun.fire_blue_npc"
                } else {
                    "Weapon_Portalgun.fire_red_npc"
                },
                Some(eye),
            ));
            if let Some(surfaces) = &self.automatic_surfaces {
                match surfaces.posed(runtime).place(eye, direction) {
                    Ok(portal) => {
                        let (fraction, _) = world.trace_point(eye, portal.center);
                        if fraction * eye.distance(portal.center) + 0.1
                            >= eye.distance(portal.center)
                        {
                            println!("automatic portal {index} from {eye:?} direction {direction:?} -> {:?}",portal.center);
                            self.place(index, portal);
                        } else {
                            runtime.unknown_inputs.insert(format!("automatic portal shot blocked by brush: {eye:?} -> {:?} fraction {fraction}",portal.center));
                        }
                    }
                    Err(error) => {
                        runtime
                            .unknown_inputs
                            .insert(format!("automatic portal shot: {error}"));
                    }
                }
            }
        }
        if let Some(events) = runtime.portal_events.as_mut() {
            self.apply_events(events);
            if std::mem::take(&mut runtime.cleanse_portals) {
                if self.portals.iter().any(Option::is_some) {
                    println!("Portal cleanser fizzled active portal pair");
                }
                self.portals = [None, None];
                self.names = [None, None];
            }
            world.portals = self.portals;
        }
    }
    pub fn fire_player(
        &mut self,
        index: usize,
        eye: Vec3,
        direction: Vec3,
        runtime: &crate::entity::Runtime,
        world: &mut crate::play::World,
    ) -> Result<Portal, String> {
        self.fire_player_with_facing(index, eye, direction, direction, runtime, world)
    }
    pub fn fire_player_with_facing(
        &mut self,
        index: usize,
        eye: Vec3,
        direction: Vec3,
        facing: Vec3,
        runtime: &crate::entity::Runtime,
        world: &mut crate::play::World,
    ) -> Result<Portal, String> {
        let portal = self.player_target(index, eye, direction, facing, runtime, world)?;
        self.place(index, portal);
        world.portals = self.portals;
        println!("player portal {index}: {portal:?}");
        Ok(portal)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn launch_player(
        &mut self,
        index: usize,
        eye: Vec3,
        direction: Vec3,
        facing: Vec3,
        muzzle: Vec3,
        runtime: &crate::entity::Runtime,
        world: &crate::play::World,
    ) -> Result<(), String> {
        if !runtime
            .portal_inventory
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            return Err("portal channel not acquired".into());
        }
        if self.shots.len() >= 64 {
            return Err("too many portal shots in flight".into());
        }
        let end = eye + direction.normalize_or_zero() * 8192.;
        let (fraction, _) = world.trace_point(eye, end);
        let direction = direction.normalize_or_zero();
        let posed = self.automatic_surfaces.as_ref().map(|s| s.posed(runtime));
        let surface_distance = posed
            .as_ref()
            .into_iter()
            .flat_map(|s| &s.faces)
            .filter_map(|face| {
                let denominator = face.normal.dot(direction);
                if denominator >= -0.0001 {
                    return None;
                }
                let distance = face.normal.dot(*face.points.first()? - eye) / denominator;
                (distance > 0. && face.contains(eye + direction * distance)).then_some(distance)
            })
            .fold(8192. * fraction, f32::min);
        let placement = self.player_target(index, eye, direction, facing, runtime, world);
        let target = placement
            .as_ref()
            .map(|p| p.center)
            .unwrap_or(eye + direction * surface_distance);
        self.shots.push(Shot {
            channel: index,
            muzzle,
            target,
            age: 0.,
            duration: (muzzle.distance(target) / 3000.).max(0.06),
            rejection: placement.err(),
            eye,
            direction,
            facing,
        });
        Ok(())
    }
    pub fn advance_shots(
        &mut self,
        dt: f32,
        runtime: &crate::entity::Runtime,
        world: &mut crate::play::World,
    ) {
        self.impacts
            .iter_mut()
            .for_each(|impact| impact.2 += dt.max(0.));
        self.impacts.retain(|impact| impact.2 < 0.3);
        let mut shots = std::mem::take(&mut self.shots);
        shots.sort_by(|a, b| (a.duration - a.age).total_cmp(&(b.duration - b.age)));
        for mut shot in shots {
            shot.age += dt.max(0.);
            if shot.age >= shot.duration {
                if shot.age - shot.duration < 0.3 {
                    self.impacts
                        .push((shot.channel, shot.target, shot.age - shot.duration));
                }
                if let Some(error) = shot.rejection {
                    println!("portal impact rejected: {error}");
                } else if let Err(error) = self.fire_player_with_facing(
                    shot.channel,
                    shot.eye,
                    shot.direction,
                    shot.facing,
                    runtime,
                    world,
                ) {
                    println!("portal impact rejected: {error}");
                }
            } else {
                self.shots.push(shot);
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn player_target(
        &self,
        index: usize,
        eye: Vec3,
        direction: Vec3,
        facing: Vec3,
        runtime: &crate::entity::Runtime,
        world: &crate::play::World,
    ) -> Result<Portal, String> {
        if !runtime
            .portal_inventory
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            return Err("portal channel not acquired".into());
        }
        let portal = self
            .automatic_surfaces
            .as_ref()
            .ok_or("portal surfaces unavailable")?
            .posed(runtime)
            .place_with_facing(eye, direction, facing)?;
        let (fraction, _) = world.trace_point(eye, portal.center);
        if fraction * eye.distance(portal.center) + 0.1 < eye.distance(portal.center) {
            return Err("portal shot blocked by brush".into());
        }
        if self.portals[1 - index].is_some_and(|p| {
            p.normal.dot(portal.normal) > 0.99 && p.center.distance(portal.center) < 110.
        }) {
            return Err("portal overlaps paired portal".into());
        }
        Ok(portal)
    }
    pub fn cancel_shots(&mut self) {
        self.shots.clear();
        self.impacts.clear();
    }
    /// Camera-facing additive ribbons. No Source particle-system parity is implied.
    pub fn shot_vertices(&self, eye: Vec3) -> Vec<crate::Vertex> {
        let mut vertices = Vec::new();
        for shot in &self.shots {
            let head = shot.position();
            let direction = (shot.target - shot.muzzle).normalize_or_zero();
            let tail = head - direction * head.distance(shot.muzzle).min(110.);
            let side = direction
                .cross(eye - head)
                .try_normalize()
                .unwrap_or(Vec3::Y);
            let color = if shot.channel == 0 {
                [0.05, 0.32, 1.]
            } else {
                [1., 0.30, 0.025]
            };
            for (width, color) in [(5., color), (0.8, [0.85, 0.92, 1.])] {
                for sign in [-1., 1.] {
                    let points = [
                        (tail, [0.; 3]),
                        (head, color),
                        (head + side * width * sign, [0.; 3]),
                        (tail + side * width * sign, [0.; 3]),
                    ];
                    for i in [0, 1, 2, 0, 2, 3] {
                        vertices.push(crate::Vertex {
                            position: points[i].0.to_array(),
                            color: points[i].1,
                        });
                    }
                }
            }
            let toward = (eye - head).normalize_or_zero();
            let head_side = toward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::Y);
            let head_up = head_side.cross(toward).normalize_or_zero();
            for (radius, tint) in [(7., color), (2., [0.85, 0.92, 1.])] {
                for i in 0..24 {
                    let a = i as f32 * std::f32::consts::TAU / 24.;
                    let b = (i + 1) as f32 * std::f32::consts::TAU / 24.;
                    for (point, color) in [
                        (head, tint),
                        (
                            head + (head_side * a.cos() + head_up * a.sin()) * radius,
                            [0.; 3],
                        ),
                        (
                            head + (head_side * b.cos() + head_up * b.sin()) * radius,
                            [0.; 3],
                        ),
                    ] {
                        vertices.push(crate::Vertex {
                            position: point.to_array(),
                            color,
                        });
                    }
                }
            }
            // Initial energy bloom at the muzzle, fading over the first 60 ms.
            if shot.age < 0.06 {
                let up = side.cross(direction).normalize_or_zero();
                let radius = 6. * (1. - shot.age / 0.06);
                for i in 0..16 {
                    let a = i as f32 * std::f32::consts::TAU / 16.;
                    let b = (i + 1) as f32 * std::f32::consts::TAU / 16.;
                    for p in [
                        shot.muzzle,
                        shot.muzzle + (side * a.cos() + up * a.sin()) * radius,
                        shot.muzzle + (side * b.cos() + up * b.sin()) * radius,
                    ] {
                        vertices.push(crate::Vertex {
                            position: p.to_array(),
                            color,
                        });
                    }
                }
            }
        }
        for &(channel, center, age) in &self.impacts {
            let toward = (eye - center).normalize_or_zero();
            let side = toward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::Y);
            let up = side.cross(toward).normalize_or_zero();
            let radius = 8. + age * 110.;
            let strength = (1. - age / 0.3).max(0.);
            let color = if channel == 0 {
                [0.05 * strength, 0.32 * strength, strength]
            } else {
                [strength, 0.30 * strength, 0.025 * strength]
            };
            let center = center + toward * 1.;
            for i in 0..32 {
                let a = i as f32 * std::f32::consts::TAU / 32.;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 32.;
                let a = side * a.cos() + up * a.sin();
                let b = side * b.cos() + up * b.sin();
                let points = [
                    center + a * radius,
                    center + b * radius,
                    center + b * (radius + 3.),
                    center + a * (radius + 3.),
                ];
                for i in [0, 1, 2, 0, 2, 3] {
                    vertices.push(crate::Vertex {
                        position: points[i].to_array(),
                        color,
                    });
                }
            }
        }
        vertices
    }
    pub fn place(&mut self, index: usize, portal: Portal) {
        self.portals[index] = Some(portal);
        self.names[index] = None;
    }
    pub fn apply_events(&mut self, events: &mut Vec<(String, bool)>) {
        for (name, active) in events.drain(..) {
            if let Some(&(index, portal)) = self.authored.get(&name) {
                if active {
                    // Map entities specify a desired placement, not an already fitted
                    // aperture. Use the same connected surface patch as fired portals.
                    let fitted = self
                        .automatic_surfaces
                        .as_ref()
                        .map(|s| s.fit_authored(portal))
                        .unwrap_or(Ok(portal));
                    if let Ok(portal) = fitted {
                        self.portals[index] = Some(portal);
                        self.names[index] = Some(name);
                    } else {
                        eprintln!("authored fit rejected {name} {portal:?}");
                        self.portals[index] = None;
                        self.names[index] = None;
                    }
                } else if self.names[index].as_deref() == Some(&name) {
                    self.portals[index] = None;
                    self.names[index] = None;
                }
            }
        }
    }
}
pub const HALF_WIDTH: f32 = 32.;
pub const HALF_HEIGHT: f32 = 54.;
#[derive(Clone)]
pub struct Surface {
    /// BSP submodel; points are model-local for nonzero models.
    pub model: usize,
    pub points: Vec<Vec3>,
    pub normal: Vec3,
    pub allowed: bool,
    pub material: String,
}
#[derive(Clone)]
pub struct Surfaces {
    pub faces: Vec<Surface>,
}
impl Surface {
    fn contains(&self, p: Vec3) -> bool {
        let mut sign = 0f32;
        for j in 0..self.points.len() {
            let a = self.points[j];
            let b = self.points[(j + 1) % self.points.len()];
            let d = (b - a).cross(p - a).dot(self.normal);
            if d.abs() < 0.01 {
                continue;
            }
            if sign * d < -0.01 {
                return false;
            }
            sign = d;
        }
        true
    }
}
impl Surfaces {
    /// Query the same resolved parent hierarchy used by brush collision/rendering.
    /// Local submodel faces must never participate at their unposed coordinates.
    pub fn posed(&self, runtime: &crate::entity::Runtime) -> Self {
        let (origins, rotations) = runtime.model_transforms();
        Self {
            faces: self
                .faces
                .iter()
                .filter_map(|face| {
                    let mut posed = face.clone();
                    if face.model != 0 {
                        if !runtime.visible.get(face.model).copied().unwrap_or(false) {
                            return None;
                        }
                        let origin = *origins.get(face.model)?;
                        let rotation = *rotations.get(face.model)?;
                        posed
                            .points
                            .iter_mut()
                            .for_each(|p| *p = rotation * *p + origin);
                        posed.normal = rotation * posed.normal;
                    }
                    posed.model = 0;
                    Some(posed)
                })
                .collect(),
        }
    }
    pub fn load(b: &[u8], fs: &Assets) -> Result<Self, String> {
        crate::load(b)?;
        let ii = |a: &[u8], o: usize| i32::from_le_bytes(a[o..o + 4].try_into().unwrap());
        let uu = |a: &[u8], o: usize| u16::from_le_bytes(a[o..o + 2].try_into().unwrap()) as usize;
        let ff = |a: &[u8], o: usize| f32::from_le_bytes(a[o..o + 4].try_into().unwrap());
        let vertices = crate::lump(b, 3, 12)?
            .chunks_exact(12)
            .map(|a| [ff(a, 0), ff(a, 4), ff(a, 8)])
            .collect::<Vec<_>>();
        let edges = crate::lump(b, 12, 4)?
            .chunks_exact(4)
            .map(|a| [uu(a, 0) as u16, uu(a, 2) as u16])
            .collect::<Vec<_>>();
        let surfedges = crate::lump(b, 13, 4)?
            .chunks_exact(4)
            .map(|a| ii(a, 0))
            .collect::<Vec<_>>();
        let planes = crate::lump(b, 1, 20)?;
        let info = crate::lump(b, 6, 72)?;
        let data = crate::lump(b, 2, 32)?;
        let names = crate::lump(b, 43, 1)?;
        let table = crate::lump(b, 44, 4)?;
        let model = crate::lump(b, 14, 48)?;
        let mut owners = vec![None; crate::lump(b, 7, 56)?.len() / 56];
        let entities = crate::entity::parse_entities(crate::lump(b, 0, 1)?)?;
        for (id, m) in model.chunks_exact(48).enumerate() {
            let solid = id == 0
                || entities.iter().any(|e| {
                    e.model().ok().flatten() == Some(id)
                        && matches!(
                            e.get("classname"),
                            Some(
                                "func_brush"
                                    | "func_door"
                                    | "func_door_rotating"
                                    | "func_button"
                                    | "func_movelinear"
                                    | "func_tracktrain"
                            )
                        )
                        && !(e.get("classname") == Some("func_brush")
                            && e.get("Solidity") == Some("1"))
                        && !(matches!(
                            e.get("classname"),
                            Some("func_door" | "func_door_rotating" | "func_tracktrain")
                        ) && e
                            .get("spawnflags")
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(0)
                            & 8
                            != 0)
                });
            if solid {
                let first = ii(m, 40) as usize;
                let count = ii(m, 44) as usize;
                for owner in owners
                    .get_mut(first..first + count)
                    .ok_or("portal model face range")?
                {
                    *owner = Some(id);
                }
            }
        }
        let mut faces = vec![];
        for (index, face) in crate::lump(b, 7, 56)?.chunks_exact(56).enumerate() {
            let Some(model) = owners[index] else { continue };
            let ti = uu(face, 10);
            if ti == 65535 {
                continue;
            }
            let inf = info
                .get(ti * 72..ti * 72 + 72)
                .ok_or("portal texinfo range")?;
            let td = ii(inf, 68) as usize;
            let td = data
                .get(td * 32..td * 32 + 32)
                .ok_or("portal texdata range")?;
            let nt = ii(td, 12) as usize;
            let nt = table.get(nt * 4..nt * 4 + 4).ok_or("portal name table")?;
            let start = ii(nt, 0) as usize;
            let name = names.get(start..).ok_or("portal material name")?;
            let end = name
                .iter()
                .position(|&n| n == 0)
                .ok_or("portal name terminator")?;
            let material = String::from_utf8_lossy(&name[..end]).to_lowercase();
            // Conservative Portal subset: only authored modular concrete. Other games' surface rules are unknown.
            let mut allowed = (material.starts_with("concrete/concrete_modular_")
                || material.starts_with("concrete/concrete_bts_modular_"))
                && ii(inf, 64) & (0x80 | 0x4 | 0x2 | 0x20 | 0x200) == 0
                && uu(face, 12) == 65535;
            if allowed {
                let vmt = fs.read(&format!("materials/{material}.vmt"))?;
                let text = String::from_utf8_lossy(&vmt).to_lowercase();
                if text.contains("noportal") {
                    allowed = false;
                }
            }
            let start = ii(face, 4) as usize;
            let n = uu(face, 8);
            let points = crate::polygon(
                &vertices,
                &edges,
                surfedges.get(start..start + n).ok_or("portal edges")?,
            )?
            .into_iter()
            .map(Vec3::from)
            .collect::<Vec<_>>();
            if points.len() < 3 {
                continue;
            }
            let pl = planes
                .get(uu(face, 0) * 20..uu(face, 0) * 20 + 20)
                .ok_or("portal plane")?;
            let plane_normal = Vec3::new(ff(pl, 0), ff(pl, 4), ff(pl, 8));
            let normal = (1..points.len() - 1)
                .map(|n| -(points[n] - points[0]).cross(points[n + 1] - points[0]))
                .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
                .ok_or("degenerate portal face")?
                .normalize_or_zero();
            if normal.length_squared() < 0.5 {
                eprintln!(
                    "portal surface skipped: zero-area face {material} at {:?}",
                    points[0]
                );
                continue;
            }
            if normal.dot(plane_normal).abs() < 0.99 {
                return Err(format!("portal face plane/winding mismatch {material} plane {plane_normal:?} winding {normal:?} at {:?}",points[0]));
            }
            faces.push(Surface {
                model,
                points,
                normal,
                allowed,
                material,
            });
        }
        Ok(Self { faces })
    }
    pub fn place(&self, eye: Vec3, direction: Vec3) -> Result<Portal, String> {
        self.place_with_facing(eye, direction, direction)
    }
    /// Facing supplies yaw when an exactly vertical ray has no horizontal heading.
    pub fn place_with_facing(
        &self,
        eye: Vec3,
        direction: Vec3,
        facing: Vec3,
    ) -> Result<Portal, String> {
        let direction = direction.normalize_or_zero();
        let mut best = 8192.;
        let mut hit = None;
        for face in self.faces.iter().filter(|face| face.model == 0) {
            let d = face.normal.dot(direction);
            if d >= -0.00001 {
                continue;
            }
            let t = face.normal.dot(face.points[0] - eye) / d;
            if t >= 0. && t < best && face.contains(eye + direction * t) {
                best = t;
                hit = Some(face);
            }
        }
        let face = hit.ok_or("no world surface")?;
        if !face.allowed {
            return Err(format!("nonportalable material {}", face.material));
        }
        let center = eye + direction * best;
        if self.faces.iter().any(|s| {
            s.model == 0
                && !s.allowed
                && !s.material.starts_with("tools/")
                && s.normal.dot(face.normal) > 0.999
                && s.normal.dot(center - s.points[0]).abs() < 0.1
                && s.contains(center)
        }) {
            return Err("nonportalable overlapping face".into());
        }
        let portal = Portal::new(
            center,
            face.normal,
            if face.normal.z.abs() > 0.99 {
                // Portal top points away from the shooter along their horizontal
                // heading. Portal::new projects this into the actual hit plane;
                // right = up x normal keeps floor and ceiling right-handed.
                let horizontal = |v: Vec3| Vec3::new(v.x, v.y, 0.);
                let heading = horizontal(direction);
                let fallback = horizontal(facing);
                if heading.length_squared() > 1e-10 {
                    heading.normalize()
                } else if fallback.length_squared() > 1e-10 {
                    fallback.normalize()
                } else {
                    Vec3::Y
                }
            } else {
                Vec3::Z
            },
        );
        self.fit_on_patch(portal, face)
    }

    /// Resolve a map-authored pose against its supporting BSP patch.
    pub fn fit_authored(&self, portal: Portal) -> Result<Portal, String> {
        // Existing correctly oriented authored placements use Source's oval
        // footprint, not the conservative fired-portal rectangle. Do not bump
        // these a second time (notably the floor-adjacent chamber00 portals).
        if self.faces.iter().any(|s| {
            s.model == 0
                && s.allowed
                && s.normal.dot(portal.normal) > 0.9999
                && s.normal.dot(portal.center - s.points[0]).abs() < 0.1
                && s.contains(portal.center)
        }) {
            return Ok(portal);
        }
        let mut fits = self
            .faces
            .iter()
            .filter(|s| {
                s.model == 0
                    && s.allowed
                    && s.normal.dot(portal.normal).abs() < 0.01
                    && s.normal.dot(portal.center - s.points[0]).abs() < 0.1
                    && s.contains(portal.center)
            })
            .filter_map(|s| {
                // Resolve the actual supporting BSP plane for tangent authored poses.
                if s.normal.dot(portal.up).abs() > 0.99 {
                    return None;
                }
                let supported = Portal::new(portal.center, s.normal, portal.up);
                self.fit_on_patch(supported, s).ok()
            })
            .collect::<Vec<_>>();
        fits.sort_by(|a, b| {
            let aligned = |p: &Portal| p.normal.dot(portal.normal) > 0.9999;
            aligned(b).cmp(&aligned(a)).then_with(|| {
                a.center
                    .distance_squared(portal.center)
                    .total_cmp(&b.center.distance_squared(portal.center))
            })
        });
        // Some authored portals live on moving models or are deliberately
        // floating (the relaxation-vault pair). World-face fitting cannot
        // validate these; retain their entity transform rather than fizzle them.
        Ok(fits.into_iter().next().unwrap_or(portal))
    }

    /// Bounded rectangular approximation of Source bumping, not Valve's oval solver.
    /// Never changes the hit plane, roll, or material selected by the original ray.
    fn fit_on_patch(&self, portal: Portal, hit: &Surface) -> Result<Portal, String> {
        let coplanar = |s: &&Surface| {
            s.normal.dot(portal.normal) > 0.9999
                && s.normal.dot(portal.center - s.points[0]).abs() < 0.05
        };
        let nearby: Vec<_> = self
            .faces
            .iter()
            .filter(|s| s.model == 0)
            .filter(coplanar)
            .collect();
        // Only the connected eligible patch containing the actual impact may fit.
        let mut patch = vec![hit];
        loop {
            let old = patch.len();
            for s in nearby.iter().copied().filter(|s| s.allowed) {
                if patch.iter().any(|p| std::ptr::eq(*p, s)) {
                    continue;
                }
                if patch.iter().any(|p| {
                    s.points.iter().any(|v| p.contains(*v))
                        || p.points.iter().any(|v| s.contains(*v))
                }) {
                    patch.push(s);
                }
            }
            if patch.len() == old {
                break;
            }
        }
        let valid = |p: Portal| {
            let rectangle = vec![
                p.center - p.right() * HALF_WIDTH - p.up * HALF_HEIGHT,
                p.center + p.right() * HALF_WIDTH - p.up * HALF_HEIGHT,
                p.center + p.right() * HALF_WIDTH + p.up * HALF_HEIGHT,
                p.center - p.right() * HALF_WIDTH + p.up * HALF_HEIGHT,
            ];
            // Polygon subtraction checks the entire rectangle, including sub-sample gaps.
            let mut uncovered = vec![rectangle.clone()];
            for s in &patch {
                uncovered = uncovered
                    .into_iter()
                    .flat_map(|poly| subtract_face(poly, s))
                    .collect();
                if uncovered.is_empty() {
                    break;
                }
                if uncovered.len() > 256 {
                    return false;
                }
            }
            uncovered.is_empty()
                && !nearby.iter().any(|s| {
                    !s.allowed
                        && !s.material.starts_with("tools/")
                        && polygon_face_overlap(rectangle.clone(), s)
                })
        };
        if valid(portal) {
            return Ok(portal);
        }
        // The nearest point of a convex feasible region is a projection onto an
        // inset edge or an intersection of two inset edges. Include both; union
        // seams may produce redundant candidates, rejected by full validation.
        let mut lines = Vec::new();
        for s in &patch {
            let centroid = s.points.iter().copied().sum::<Vec3>() / s.points.len() as f32;
            for i in 0..s.points.len() {
                let a = s.points[i];
                let edge = s.points[(i + 1) % s.points.len()] - a;
                let mut n = portal.normal.cross(edge).normalize_or_zero();
                if n.dot(centroid - a) < 0. {
                    n = -n;
                }
                let axis = glam::Vec2::new(n.dot(portal.right()), n.dot(portal.up));
                let distance = n.dot(a - portal.center)
                    + axis.x.abs() * HALF_WIDTH
                    + axis.y.abs() * HALF_HEIGHT;
                if axis.length_squared() > 0.5 {
                    lines.push((axis, distance));
                }
            }
        }
        lines.truncate(128);
        let mut candidates = Vec::new();
        for (i, &(a, b)) in lines.iter().enumerate() {
            candidates.push(a * b);
            for &(c, d) in &lines[..i] {
                let det = a.perp_dot(c);
                if det.abs() > 1e-5 {
                    candidates.push(glam::Vec2::new(b * c.y - a.y * d, a.x * d - b * c.x) / det);
                }
            }
        }
        candidates.retain(|d| {
            d.is_finite() && d.x.abs() <= HALF_WIDTH + 0.01 && d.y.abs() <= HALF_HEIGHT + 0.01
        });
        candidates.sort_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
        candidates.dedup_by(|a, b| a.distance_squared(*b) < 1e-6);
        for delta in candidates.into_iter().take(4096) {
            let p = Portal {
                center: portal.center + portal.right() * delta.x + portal.up * delta.y,
                ..portal
            };
            if valid(p) {
                return Ok(p);
            }
        }
        Err("portal footprint crosses edge or forbidden surface".into())
    }
}

// Split a convex polygon at a face edge; discard numerical zero-area slivers.
fn split_polygon(poly: Vec<Vec3>, a: Vec3, n: Vec3) -> (Vec<Vec3>, Vec<Vec3>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        let d = n.dot(p - a);
        let e = n.dot(q - a);
        if d >= 0. {
            inside.push(p);
        } else {
            outside.push(p);
        }
        if (d < 0.) != (e < 0.) {
            let v = p + (q - p) * (d / (d - e));
            inside.push(v);
            outside.push(v);
        }
    }
    for polygon in [&mut inside, &mut outside] {
        let area = if polygon.len() < 3 {
            0.
        } else {
            (1..polygon.len() - 1)
                .map(|i| {
                    (polygon[i] - polygon[0])
                        .cross(polygon[i + 1] - polygon[0])
                        .length()
                })
                .sum::<f32>()
        };
        if area < 0.001 {
            polygon.clear();
        }
    }
    (inside, outside)
}
fn face_inward(s: &Surface, i: usize) -> Vec3 {
    let centroid = s.points.iter().copied().sum::<Vec3>() / s.points.len() as f32;
    let a = s.points[i];
    let mut n = s
        .normal
        .cross(s.points[(i + 1) % s.points.len()] - a)
        .normalize_or_zero();
    if n.dot(centroid - a) < 0. {
        n = -n;
    }
    n
}
fn subtract_face(mut poly: Vec<Vec3>, s: &Surface) -> Vec<Vec<Vec3>> {
    let mut remainder = Vec::new();
    for i in 0..s.points.len() {
        let (inside, outside) = split_polygon(poly, s.points[i], face_inward(s, i));
        if !outside.is_empty() {
            remainder.push(outside);
        }
        poly = inside;
        if poly.is_empty() {
            break;
        }
    }
    remainder
}
fn polygon_face_overlap(mut poly: Vec<Vec3>, s: &Surface) -> bool {
    for i in 0..s.points.len() {
        poly = split_polygon(poly, s.points[i], face_inward(s, i)).0;
        if poly.is_empty() {
            return false;
        }
    }
    true
}

use glam::{Mat3, Vec3};
#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub center: Vec3,
    pub normal: Vec3,
    pub up: Vec3,
}
impl Portal {
    pub fn new(center: Vec3, normal: Vec3, up: Vec3) -> Self {
        let normal = normal.normalize();
        Self {
            center,
            normal,
            up: (up - normal * normal.dot(up)).normalize(),
        }
    }
    pub fn linked_view(
        self,
        to: Self,
        eye: Vec3,
        forward: Vec3,
        up: Vec3,
    ) -> (Vec3, Vec3, Vec3, glam::Vec4) {
        (
            self.map_point(to, eye),
            self.map_vector(to, forward),
            self.map_vector(to, up),
            to.normal.extend(-to.normal.dot(to.center) - 0.02),
        )
    }
    pub fn fits(self, center: Vec3, extent: Vec3) -> bool {
        let d = center - self.center;
        let x = (d.dot(self.right()).abs() + self.right().abs().dot(extent)) / HALF_WIDTH;
        let y = (d.dot(self.up).abs() + self.up.abs().dot(extent)) / HALF_HEIGHT;
        x < 0.99 && y < 0.99
    }
    pub fn right(self) -> Vec3 {
        self.up.cross(self.normal)
    }
    pub fn basis(self) -> Mat3 {
        Mat3::from_cols(self.right(), self.up, self.normal)
    }
    pub fn map_vector(self, to: Self, v: Vec3) -> Vec3 {
        to.basis() * (Vec3::new(-1., 1., -1.) * (self.basis().transpose() * v))
    }
    pub fn map_point(self, to: Self, p: Vec3) -> Vec3 {
        to.center + self.map_vector(to, p - self.center)
    }
}
