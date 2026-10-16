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

pub struct Game {
    pub automatic_surfaces: Option<Surfaces>,
    pub portals: [Option<Portal>; 2],
    names: [Option<String>; 2],
    authored: std::collections::HashMap<String, (usize, Portal)>,
}
impl Game {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let mut game = Self {
            automatic_surfaces: None,
            portals: [None; 2],
            names: [None, None],
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
        for (index, eye, direction) in std::mem::take(&mut runtime.portal_shots) {
            if let Some(surfaces) = &self.automatic_surfaces {
                match surfaces.place(eye, direction) {
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
        self.place(index, portal);
        world.portals = self.portals;
        println!("player portal {index}: {portal:?}");
        Ok(portal)
    }
    pub fn place(&mut self, index: usize, portal: Portal) {
        self.portals[index] = Some(portal);
        self.names[index] = None;
    }
    pub fn apply_events(&mut self, events: &mut Vec<(String, bool)>) {
        for (name, active) in events.drain(..) {
            if let Some(&(index, portal)) = self.authored.get(&name) {
                if active {
                    self.portals[index] = Some(portal);
                    self.names[index] = Some(name);
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
        let first = ii(model, 40) as usize;
        let count = ii(model, 44) as usize;
        let mut faces = vec![];
        for face in crate::lump(b, 7, 56)?
            .chunks_exact(56)
            .skip(first)
            .take(count)
        {
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
            let mut allowed = material.starts_with("concrete/concrete_modular_")
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
        for face in &self.faces {
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
            !s.allowed
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
        // Validate a complete enclosing rectangle over the union of coplanar eligible world polygons.
        // This is deliberately stricter than Source's oval bump/fitting algorithm.
        for x in -8..=8 {
            for y in -14..=14 {
                let p = center
                    + portal.right() * (x as f32 * 4.)
                    + portal.up * (y as f32 * HALF_HEIGHT / 14.);
                if !self.faces.iter().any(|s| {
                    s.allowed
                        && s.normal.dot(portal.normal) > 0.9999
                        && s.normal.dot(p - s.points[0]).abs() < 0.05
                        && s.contains(p)
                }) {
                    return Err("portal footprint crosses edge or forbidden surface".into());
                }
            }
        }
        Ok(portal)
    }
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
