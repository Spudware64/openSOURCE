use glam::Vec3;
use winit::keyboard::KeyCode;
// Source SDK public/bspflags.h. Portal shots deliberately pass through grates;
// player hulls and solid objects do not. Keep contents until query selection.
const MASK_PLAYERSOLID: i32 = 1 | 2 | 8 | 0x4000 | 0x10000 | 0x2000000;
const MASK_SHOT_PORTAL: i32 = 1 | 2 | 0x4000 | 0x2000000;
const MASK_SOLID: i32 = 1 | 2 | 8 | 0x4000 | 0x2000000;
/// Render-only one-tick interpolation. Never feeds a visual position into physics.
#[derive(Default)]
pub struct RenderPosition {
    previous: Option<(Vec3, u64)>,
}
impl RenderPosition {
    pub fn begin_step(&mut self, player: &Player) {
        self.previous = Some((player.position, player.portal_crossings as u64));
    }
    pub fn sample(&self, player: &Player, alpha: f32) -> Vec3 {
        match self.previous {
            Some((position, crossings)) if crossings == player.portal_crossings as u64 => {
                position.lerp(player.position, alpha.clamp(0., 1.))
            }
            _ => player.position,
        }
    }
}
#[derive(Default)]
pub struct Controls {
    pub keys: std::collections::HashSet<KeyCode>,
    pub captured: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    ReleaseCursor,
    Reset,
    ToggleNoclip,
    Use,
    Throw,
}
impl Controls {
    pub fn press(&mut self, key: KeyCode) -> Action {
        self.keys.insert(key);
        match key {
            KeyCode::Escape => {
                self.keys.clear();
                self.captured = false;
                Action::ReleaseCursor
            }
            KeyCode::KeyR => Action::Reset,
            KeyCode::KeyN => Action::ToggleNoclip,
            KeyCode::KeyE => Action::Use,
            KeyCode::KeyF => Action::Throw,
            _ => Action::None,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}
pub struct World {
    pub portals: [Option<crate::portal::Portal>; 2],
    pub brushes: Vec<Vec<Plane>>,
    brush_contents: Vec<i32>,
    model_contents: Vec<Vec<i32>>,
    pub(crate) rigid: Vec<Vec<Plane>>,
    pub(crate) rigid_ids: Vec<usize>,
    pub spawn: Vec3,
    pub yaw: f32,
    pub model_brushes: Vec<Vec<Vec<Plane>>>,
    pub model_origins: Vec<Vec3>,
    pub model_solid: Vec<bool>,
    floor_support: Vec<Vec<Vec<Plane>>>,
    mesh_sides: Vec<Vec<crate::mesh_collision::Group>>,
    train_models: std::collections::HashSet<usize>,
}
impl World {
    /// Monotonic Parry narrowphase invocation count on this simulation thread.
    /// Compare before/after movement; this is diagnostic, not per-World state.
    pub fn player_narrowphase_queries(&self) -> u64 { crate::player_query::queries() }
    /// Rigid upward triangle support from actual posed MDL geometry. This is not PHY collision.
    pub fn bind_scene(&mut self, scene: &crate::scene::Scene) {
        if self.model_origins.len() < scene.origins.len() {
            self.model_origins
                .extend_from_slice(&scene.origins[self.model_origins.len()..]);
        }
        self.floor_support
            .resize_with(scene.origins.len(), Vec::new);
        self.mesh_sides.resize_with(scene.origins.len(), Vec::new);
        for instance in &scene.animations {
            if !instance.solid || instance.skeleton.clips.len() != 1 {
                continue;
            }
            let mut sides = Vec::new();
            for tri in scene.vertices[instance.range.clone()].chunks_exact(3) {
                let points = [
                    Vec3::from(tri[0].position),
                    Vec3::from(tri[1].position),
                    Vec3::from(tri[2].position),
                ];
                let normal = Vec3::from(tri[0].normal)
                    + Vec3::from(tri[1].normal)
                    + Vec3::from(tri[2].normal);
                let geometric_normal = (points[1] - points[0]).cross(points[2] - points[0]);
                if geometric_normal.length_squared() > 0.001
                    && geometric_normal.normalize().z.abs() < 0.7
                    && self.train_models.contains(&(tri[0].model as usize))
                {
                    sides.push(crate::mesh_collision::Triangle::new(points));
                }
                if normal.normalize_or_zero().z < 0.99
                    || (points[0].z - points[1].z).abs() > 0.01
                    || (points[0].z - points[2].z).abs() > 0.01
                {
                    continue;
                }
                let center = (points[0] + points[1] + points[2]) / 3.;
                let mut brush = vec![
                    Plane {
                        normal: Vec3::Z,
                        distance: points[0].z,
                    },
                    Plane {
                        normal: -Vec3::Z,
                        distance: -points[0].z + 0.25,
                    },
                ];
                for i in 0..3 {
                    let edge = points[(i + 1) % 3] - points[i];
                    let mut n = Vec3::new(edge.y, -edge.x, 0.).normalize_or_zero();
                    if n.length_squared() < 0.5 {
                        continue;
                    }
                    if n.dot(center - points[i]) > 0. {
                        n = -n;
                    }
                    brush.push(Plane {
                        normal: n,
                        distance: n.dot(points[i]),
                    });
                }
                if brush.len() == 5 {
                    // Triangle-edge halfspaces alone are not the exact swept-box
                    // footprint: they invent support beyond acute corners. Include
                    // the box-axis separating planes (the 2-D triangle/AABB SAT).
                    let lo = points[0].min(points[1]).min(points[2]);
                    let hi = points[0].max(points[1]).max(points[2]);
                    for (normal, distance) in [
                        (Vec3::X, hi.x),
                        (-Vec3::X, -lo.x),
                        (Vec3::Y, hi.y),
                        (-Vec3::Y, -lo.y),
                    ] {
                        brush.push(Plane { normal, distance });
                    }
                    self.floor_support[tri[0].model as usize].push(brush);
                }
            }
            if !sides.is_empty() {
                let model = scene.vertices[instance.range.start].model as usize;
                self.mesh_sides[model].push(crate::mesh_collision::Group::new(sides));
            }
        }
    }
    pub fn from_brushes(brushes: Vec<Vec<Plane>>) -> Self {
        Self {
            portals: [None; 2],
            rigid: vec![],
            rigid_ids: vec![],
            floor_support: vec![vec![]],
            mesh_sides: vec![vec![]],
            train_models: Default::default(),
            model_brushes: vec![brushes.clone()],
            model_contents: vec![vec![1; brushes.len()]],
            brush_contents: vec![1; brushes.len()],
            model_origins: vec![Vec3::ZERO],
            model_solid: vec![true],
            brushes,
            spawn: Vec3::ZERO,
            yaw: 0.,
        }
    }
    pub fn overlaps(&self, position: Vec3, height: f32) -> bool {
        let center = position + Vec3::Z * height * 0.5;
        let extent = Vec3::new(16., 16., height * 0.5);
        self.brushes
            .iter()
            .chain(self.rigid.iter())
            .enumerate()
            .any(|(index, b)| {
                !(index < self.model_brushes[0].len() && self.aperture(b, center, center, extent))
                    && b.iter().all(|p| {
                        p.normal.dot(center) - p.distance - p.normal.abs().dot(extent) < -0.001
                    })
            })
    }
    pub fn parse(b: &[u8]) -> Result<Self, String> {
        crate::validate(b)?;
        for id in [1, 18, 19] {
            let h = 8 + id * 16 + 8;
            if b[h..h + 4] != [0; 4] {
                return Err(format!("unsupported collision layout for lump {id}"));
            }
        }
        let i = |a: &[u8], o| i32::from_le_bytes(a[o..o + 4].try_into().unwrap());
        let f = |a: &[u8], o| f32::from_le_bytes(a[o..o + 4].try_into().unwrap());
        let planes: Vec<_> = crate::lump(b, 1, 20)?
            .chunks_exact(20)
            .map(|a| Plane {
                normal: Vec3::new(f(a, 0), f(a, 4), f(a, 8)),
                distance: f(a, 12),
            })
            .collect();
        if planes.iter().any(|p| {
            !p.normal.is_finite()
                || !p.distance.is_finite()
                || (p.normal.length() - 1.).abs() > 0.01
        }) {
            return Err("invalid collision plane".into());
        }
        let sides = crate::lump(b, 19, 8)?;
        let mut brushes = vec![];
        let mut contents = vec![];
        for a in crate::lump(b, 18, 12)?.chunks_exact(12) {
            let start = i(a, 0);
            let count = i(a, 4);
            if start < 0 || count < 0 || start as usize + count as usize > sides.len() / 8 {
                return Err("brush side range".into());
            }
            let mut brush = vec![];
            for s in sides[start as usize * 8..(start + count) as usize * 8].chunks_exact(8) {
                let n = u16::from_le_bytes(s[..2].try_into().unwrap()) as usize;
                brush.push(*planes.get(n).ok_or("brush plane range")?);
            }
            contents.push(i(a, 8));
            if i(a, 8) & MASK_PLAYERSOLID == 0 {
                brush.clear();
            }
            brushes.push(brush);
        }
        let indices = crate::models::model_brush_indices(b)?;
        let mut model_brushes = indices
            .iter()
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&n| {
                        if brushes[n].is_empty() {
                            None
                        } else {
                            Some(brushes[n].clone())
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let entities = crate::entity::parse_entities(crate::lump(b, 0, 1)?)?;
        let mut model_contents: Vec<Vec<i32>> = indices.iter().map(|ids| {
            ids.iter().filter(|&&id| !brushes[id].is_empty()).map(|&id| contents[id]).collect()
        }).collect();
        let player = entities
            .iter()
            .find(|e| e.get("classname") == Some("info_player_start"))
            .or_else(|| {
                entities.iter().find(|e| {
                    e.get("classname")
                        .is_some_and(|c| c.starts_with("info_player_"))
                })
            })
            .ok_or("no supported player spawn")?;
        let spawn = player.origin()?;
        let yaw = crate::entity::vector(player.get("angles").unwrap_or("0 0 0"))?
            .y
            .to_radians();
        let mut origins = vec![Vec3::ZERO; model_brushes.len()];
        let mut solid = vec![true; model_brushes.len()];
        let mut train_models = std::collections::HashSet::new();
        for e in &entities {
            if let Some(model) = e.model()? {
                *origins
                    .get_mut(model)
                    .ok_or("entity collision model range")? = e.origin()?;
                let class = e.get("classname").unwrap_or("");
                if class == "func_tracktrain" {
                    train_models.insert(model);
                }
                // SF_TRACKTRAIN_PASSABLE: the map's train brush is an invisible
                // transform carrier, not the visible elevator body's collision.
                let passable = ["func_tracktrain", "func_door", "func_door_rotating"]
                    .contains(&class)
                    && e.get("spawnflags")
                        .unwrap_or("0")
                        .parse::<u32>()
                        .map_err(|e| e.to_string())?
                        & 8
                        != 0;
                if passable {
                    model_brushes[model].clear();
                    model_contents[model].clear();
                }
                solid[model] = !(passable
                    || class.starts_with("trigger_")
                    || [
                        "func_areaportal",
                        "func_noportal_volume",
                        "func_portal_detector",
                        "func_illusionary",
                    ]
                    .contains(&class)
                    || class == "func_brush" && e.get("Solidity") == Some("1"));
                let rotation = crate::models::rotation(crate::entity::vector(
                    e.get("angles").unwrap_or("0 0 0"),
                )?);
                for brush in &mut model_brushes[model] {
                    for plane in brush {
                        plane.normal = rotation * plane.normal;
                    }
                }
            }
        }
        let mut world = Self {
            portals: [None; 2],
            rigid: vec![],
            rigid_ids: vec![],
            brushes: vec![],
            brush_contents: vec![],
            model_contents,
            spawn,
            yaw,
            floor_support: vec![vec![]; model_brushes.len()],
            mesh_sides: vec![vec![]; model_brushes.len()],
            train_models,
            model_brushes,
            model_origins: origins.clone(),
            model_solid: solid.clone(),
        };
        world.update_models(&origins, &solid);
        Ok(world)
    }
    pub fn update_models(&mut self, origins: &[Vec3], enabled: &[bool]) {
        self.update_transforms(origins, enabled, &[]);
    }
    pub fn update_transforms(
        &mut self,
        origins: &[Vec3],
        enabled: &[bool],
        rotations: &[glam::Mat3],
    ) {
        let _profile = crate::frame_profile::scope(11);
        crate::frame_profile::count(5, 1);
        self.model_origins = origins.to_vec();
        self.brushes.clear();
        self.brush_contents.clear();
        for (model, brushes) in self.model_brushes.iter().enumerate() {
            if !self.model_solid[model] || !enabled.get(model).copied().unwrap_or(false) {
                continue;
            }
            let origin = origins.get(model).copied().unwrap_or(Vec3::ZERO);
            let rotation = rotations
                .get(model)
                .copied()
                .unwrap_or(glam::Mat3::IDENTITY);
            for (index, brush) in brushes.iter().enumerate() {
                // Public synthetic geometry defaults to CONTENTS_SOLID.
                let contents = self.model_contents.get(model).and_then(|v| v.get(index)).copied().unwrap_or(1);
                self.brush_contents.push(contents);
                self.brushes.push(
                    brush
                        .iter()
                        .map(|p| Plane {
                            normal: rotation * p.normal,
                            distance: p.distance + (rotation * p.normal).dot(origin),
                        })
                        .collect(),
                );
            }
        }
    }
    pub fn trace(&self, start: Vec3, end: Vec3) -> (f32, Vec3) {
        self.trace_height(start, end, 72.)
    }
    pub fn trace_height(&self, start: Vec3, end: Vec3, height: f32) -> (f32, Vec3) {
        self.trace_hull(
            start,
            end,
            Vec3::new(0., 0., height * 0.5),
            Vec3::new(16., 16., height * 0.5),
            true,
            None,
            MASK_PLAYERSOLID,
        )
    }
    pub fn trace_point(&self, start: Vec3, end: Vec3) -> (f32, Vec3) {
        self.trace_hull(start, end, Vec3::ZERO, Vec3::ZERO, true, None, MASK_SHOT_PORTAL)
    }
    pub(crate) fn trace_use(&self, start: Vec3, end: Vec3, target: usize) -> (f32, Vec3) {
        self.trace_hull(start, end, Vec3::ZERO, Vec3::ZERO, true, Some(target), MASK_SOLID)
    }
    pub fn overlaps_cuboid(&self, center: Vec3, extent: Vec3) -> bool {
        self.brushes
            .iter()
            .chain(self.rigid.iter())
            .enumerate()
            .any(|(index, b)| {
                !(index < self.model_brushes[0].len() && self.aperture(b, center, center, extent))
                    && b.iter().all(|p| {
                        p.normal.dot(center) - p.distance - p.normal.abs().dot(extent) < -0.001
                    })
            })
    }
    pub(crate) fn body_overlap(&self, id: usize, center: Vec3, extent: Vec3) -> bool {
        self.brushes
            .iter()
            .chain(self.rigid.iter())
            .enumerate()
            .any(|(index, b)| {
                if index >= self.brushes.len()
                    && self.rigid_ids.get(index - self.brushes.len()) == Some(&id)
                {
                    return false;
                }
                !(index < self.model_brushes[0].len()
                    && self.aperture_except(b, center, center, extent, Some(id)))
                    && b.iter().all(|p| {
                        p.normal.dot(center) - p.distance - p.normal.abs().dot(extent) < -0.001
                    })
            })
    }
    pub fn trace_cuboid(&self, start: Vec3, end: Vec3, extent: Vec3) -> (f32, Vec3) {
        self.trace_hull(start, end, Vec3::ZERO, extent, false, None, MASK_SOLID)
    }
    fn trace_hull(
        &self,
        start: Vec3,
        end: Vec3,
        center: Vec3,
        extent: Vec3,
        rigid: bool,
        ignore: Option<usize>,
        mask: i32,
    ) -> (f32, Vec3) {
        let mut hit = (1., Vec3::ZERO);
        for (index, brush) in self.brushes.iter().enumerate() {
            if self.brush_contents.get(index).copied().unwrap_or(1) & mask == 0 {
                continue;
            }
            // Cutting out the backing brush must retain its aperture rim. A
            // hull already inserted into the brush has no convex entry face,
            // so restoring the whole brush after a failed fit loses collision.
            if index < self.model_brushes[0].len() && extent.length_squared() >= 1. {
                if let [Some(a), Some(b)] = self.portals {
                    for (src, dst) in [(a, b), (b, a)] {
                        let d0 = src.normal.dot(start + center - src.center);
                        let d1 = src.normal.dot(end + center - src.center);
                        let reach = src.normal.abs().dot(extent) + 8.1;
                        if d0.min(d1) > reach
                            || d0.max(d1) < -reach
                            || !Self::portal_brush(brush, src, extent)
                        {
                            continue;
                        }
                        for (portal, mapped) in [(src, false), (dst, true)] {
                            for (axis, half) in [
                                (portal.right(), crate::portal::HALF_WIDTH),
                                (portal.up, crate::portal::HALF_HEIGHT),
                            ] {
                                // The destination body remains upright; its support
                                // extents are not the rotated source body's extents.
                                let limit = half * 0.99 - axis.abs().dot(extent);
                                let local_axis = if mapped {
                                    dst.map_vector(src, axis)
                                } else {
                                    axis
                                };
                                for sign in [-1., 1.] {
                                    let normal = -local_axis * sign;
                                    let mut rim = brush.clone();
                                    rim.push(Plane {
                                        normal,
                                        distance: normal.dot(src.center)
                                            - limit
                                            - normal.abs().dot(extent),
                                    });
                                    let h = Self::trace_brushes(
                                        std::slice::from_ref(&rim),
                                        start,
                                        end,
                                        center,
                                        extent,
                                    );
                                    if h.0 < hit.0 {
                                        hit = h;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if index < self.model_brushes[0].len()
                && self.aperture_except(brush, start + center, end + center, extent, ignore)
            {
                continue;
            }
            let h = Self::trace_brushes(std::slice::from_ref(brush), start, end, center, extent);
            if h.0 < hit.0 {
                hit = h;
            }
        }
        if rigid {
            for (index, brush) in self.rigid.iter().enumerate() {
                if ignore.is_some() && self.rigid_ids.get(index).copied() == ignore {
                    continue;
                }
                let h =
                    Self::trace_brushes(std::slice::from_ref(brush), start, end, center, extent);
                if h.0 < hit.0 {
                    hit = h;
                }
            }
        }
        if end.z < start.z {
            for model in 0..self.floor_support.len() {
                let origin = self.model_origins[model];
                for brush in &self.floor_support[model] {
                    let top = brush[0].distance + origin.z;
                    let feet = start.z + center.z - extent.z;
                    if feet < top - 0.001 {
                        continue;
                    }
                    let h = crate::player_query::cast(brush, start + center - origin, end + center - origin, extent);
                    if h.0 < hit.0 && h.1.z > 0.7 { hit = h; }
                }
            }
        }
        for (model, triangles) in self.mesh_sides.iter().enumerate() {
            let origin = self.model_origins[model];
            for triangle in triangles {
                let h = triangle.trace(start + center - origin, end + center - origin, extent);
                if h.0 < hit.0 {
                    hit = h;
                }
            }
        }
        hit
    }
    fn portal_brush(brush: &[Plane], p: crate::portal::Portal, extent: Vec3) -> bool {
        brush.iter().any(|s| {
            s.normal.dot(p.normal) > 0.9999
                && (s.distance - s.normal.dot(p.center)).abs() < p.normal.abs().dot(extent) + 8.1
        }) && (brush.iter().all(|s| {
            s.normal.dot(p.normal).abs() > 0.9999
                || s.normal.dot(p.center) - s.distance < s.normal.abs().dot(extent) + 0.1
        }) || {
            let front = brush.iter().find(|s| s.normal.dot(p.normal) > 0.9999);
            let back = brush.iter().find(|s| s.normal.dot(p.normal) < -0.9999);
            matches!((front,back),(Some(a),Some(b)) if a.distance+b.distance<=16.1)
        })
    }
    fn aperture(&self, brush: &[Plane], start: Vec3, end: Vec3, extent: Vec3) -> bool {
        self.aperture_except(brush, start, end, extent, None)
    }
    pub(crate) fn aperture_except(
        &self,
        brush: &[Plane],
        start: Vec3,
        end: Vec3,
        extent: Vec3,
        ignore: Option<usize>,
    ) -> bool {
        let [Some(a), Some(b)] = self.portals else {
            return false;
        };
        if extent.length_squared() < 1. {
            return false;
        }
        [(a, b), (b, a)].into_iter().any(|(p, dst)| {
            let a = p.normal.dot(start - p.center);
            let b = p.normal.dot(end - p.center);
            let reach = p.normal.abs().dot(extent) + 2.;
            let t = if (a - b).abs() > 0.0001 {
                (a / (a - b)).clamp(0., 1.)
            } else {
                0.
            };
            let center = start.lerp(end, t);
            if a.min(b) > reach
                || a.max(b) < -reach
                || !p.fits(center, extent)
                || !Self::portal_brush(brush, p, extent)
            {
                return false;
            }
            let at = center - p.normal * p.normal.dot(center - p.center);
            let mapped = p.map_point(dst, at) + dst.normal * 0.05;
            dst.fits(mapped, extent)
                && !self
                    .brushes
                    .iter()
                    .chain(self.rigid.iter())
                    .enumerate()
                    .any(|(index, other)| {
                        if index >= self.brushes.len()
                            && ignore.is_some()
                            && self.rigid_ids.get(index - self.brushes.len()).copied() == ignore
                        {
                            return false;
                        }
                        let support = index < self.model_brushes[0].len()
                            && Self::portal_brush(other, dst, extent);
                        !support
                            && other.iter().all(|s| {
                                s.normal.dot(mapped) - s.distance - s.normal.abs().dot(extent)
                                    < -0.001
                            })
                    })
        })
    }
    pub fn support_model(&self, position: Vec3, height: f32) -> Option<usize> {
        // Carry the entity which actually wins the ground probe, not any moving
        // brush within two units below a nearer static floor. World wins ties.
        let (mut best, _) = Self::trace_brushes(
            &self.model_brushes[0],
            position + Vec3::Z * 0.1,
            position - Vec3::Z * 2.,
            Vec3::Z * height * 0.5,
            Vec3::new(16., 16., height * 0.5),
        );
        let mut support = None;
        for model in 1..self.model_brushes.len() {
            if !self.model_solid[model] {
                continue;
            }
            let origin = self.model_origins[model];
            let (f, n) = Self::trace_brushes(
                &self.model_brushes[model],
                position - origin + Vec3::Z * 0.1,
                position - origin - Vec3::Z * 2.,
                Vec3::Z * height * 0.5,
                Vec3::new(16., 16., height * 0.5),
            );
            if n.z > 0.7 && f < best {
                best = f;
                support = Some(model);
            }
        }
        for model in 0..self.floor_support.len() {
            let origin = self.model_origins[model];
            for brush in &self.floor_support[model] {
                let top = brush[0].distance + origin.z;
                if (position.z - top).abs() > 0.2 {
                    continue;
                }
                let f = ((position.z + 0.1 - top - 0.03125) / 2.1).max(0.);
                if f >= best {
                    continue;
                }
                let point = position - origin;
                if brush[2..].iter().all(|p| {
                    p.normal.dot(point) - p.distance <= p.normal.abs().dot(Vec3::new(16., 16., 0.))
                }) {
                    best = f;
                    support = (model != 0).then_some(model);
                }
            }
        }
        for (model, groups) in self.mesh_sides.iter().enumerate() {
            let origin = self.model_origins[model];
            for group in groups {
                let (f, n) = group.trace(
                    position + Vec3::Z * (height * 0.5 + 0.1) - origin,
                    position + Vec3::Z * (height * 0.5 - 2.) - origin,
                    Vec3::new(16., 16., height * 0.5),
                );
                if n.z >= 0.7 && f < best {
                    best = f;
                    support = (model != 0).then_some(model);
                }
            }
        }
        support
    }
    pub(crate) fn trace_brushes(
        brushes: &[Vec<Plane>],
        start: Vec3,
        end: Vec3,
        center: Vec3,
        extent: Vec3,
    ) -> (f32, Vec3) {
        let mut fraction = 1.;
        let mut normal = Vec3::ZERO;
        for brush in brushes {
            let h = crate::player_query::cast(brush, start + center, end + center, extent);
            if h.0 < fraction { fraction = h.0; normal = h.1; }
        }
        (fraction, normal)
    }
}
pub fn smoke_tick(frame: u64, p: &mut Player, w: &World) {
    if frame == 120 {
        p.yaw = 0.;
        p.pitch = -0.1;
    }
    let input = if (120..360).contains(&frame) {
        Vec3::X
    } else {
        Vec3::ZERO
    };
    p.step(w, input, frame == 360, 1. / 120.);
}
pub struct Player {
    pub view_up: Vec3,
    pub portal_crossings: u64,
    /// Actual most recent controller transfer (source, destination).
    pub last_portal_transfer: Option<(crate::portal::Portal, crate::portal::Portal)>,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub noclip: bool,
    pub crouched: bool,
}
impl Player {
    pub fn spawn(world: &World) -> Self {
        Self::new(world.spawn, world.yaw)
    }
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            view_up: Vec3::Z,
            portal_crossings: 0,
            last_portal_transfer: None,
            yaw,
            pitch: 0.,
            grounded: false,
            noclip: false,
            crouched: false,
        }
    }
    pub fn height(&self) -> f32 {
        if self.crouched {
            36.
        } else {
            72.
        }
    }
    pub fn eye_height(&self) -> f32 {
        if self.crouched {
            28.
        } else {
            64.
        }
    }
    pub fn set_crouch(&mut self, w: &World, crouch: bool) {
        if crouch || !w.overlaps(self.position, 72.) {
            self.crouched = crouch;
        }
    }
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
        )
    }
    pub fn step(&mut self, w: &World, input: Vec3, jump: bool, dt: f32) {
        // Roll recovery is a local camera policy, not recovered Source tuning.
        let f = self.forward();
        let target = (Vec3::Z - f * f.z).normalize_or_zero();
        let current = (self.view_up - f * self.view_up.dot(f)).normalize_or_zero();
        if current.length_squared() < 0.5 {
            self.view_up = if target.length_squared() > 0.5 {
                target
            } else {
                Vec3::Y
            };
        }
        if target.length_squared() > 0.5 && current.length_squared() > 0.5 {
            let angle = f.dot(current.cross(target)).atan2(current.dot(target));
            self.view_up = glam::Quat::from_axis_angle(f, angle.clamp(-3. * dt, 3. * dt)) * current;
        }
        let forward = Vec3::new(self.yaw.cos(), self.yaw.sin(), 0.);
        let right = Vec3::new(self.yaw.sin(), -self.yaw.cos(), 0.);
        let wish = (forward * input.x + right * input.y).normalize_or_zero()
            * if self.crouched { 90. } else { 190. };
        if self.noclip {
            self.position += (wish + Vec3::Z * input.z * 190.) * dt;
            self.velocity = Vec3::ZERO;
            return;
        }
        let (ground, n) =
            w.trace_height(self.position, self.position - Vec3::Z * 2., self.height());
        // Original server.dll FUN_1018cef0 rejects ground above 140 u/s
        // upward relative speed before its two-unit floor probe (REA evidence
        // ev_4533759265b86c5b6740e7df90d184169f72d141cd43bfb6fefccd87bd1053d1).
        // This controller carries platform displacement separately in Runtime;
        // its velocity is already relative to that support, not world lift speed.
        self.grounded = self.velocity.z <= 140. && ground < 1. && n.z >= 0.7;
        if self.grounded {
            // Bounded Source-style ground friction and projected acceleration.
            // Keep the existing speed cap; these coefficients are a local policy,
            // not a claim of recovered Valve movement tuning.
            let speed = self.velocity.truncate().length();
            if speed > 0. {
                let next = (speed - speed.max(100.) * 4. * dt).max(0.);
                self.velocity.x *= next / speed;
                self.velocity.y *= next / speed;
            }
            if wish.length_squared() > 0. {
                let direction = wish.normalize();
                let add = (wish.length() - self.velocity.dot(direction))
                    .clamp(0., 10. * wish.length() * dt);
                self.velocity += direction * add;
            }
        } else if wish.length_squared() > 0. {
            let direction = wish.normalize();
            let add = (wish.length() - self.velocity.dot(direction)).clamp(0., 1000. * dt);
            self.velocity += direction * add;
        }
        if self.grounded && self.velocity.z < 0. {
            self.velocity.z = 0.;
        }
        if jump && self.grounded {
            self.velocity.z = 270.;
            self.grounded = false;
        }
        self.velocity.z -= 600. * dt;
        let start = self.position;
        let initial_velocity = self.velocity;
        let was_grounded = self.grounded;
        let mut remaining = dt;
        for _ in 0..4 {
            let offset = Vec3::Z * self.height() * 0.5;
            let extent = Vec3::new(16., 16., self.height() * 0.5);
            if let [Some(a), Some(b)] = w.portals {
                let mut crossed = false;
                for (src, dst) in [(a, b), (b, a)] {
                    let center = self.position + offset;
                    let end = center + self.velocity * remaining;
                    let d0 = src.normal.dot(center - src.center);
                    let d1 = src.normal.dot(end - src.center);
                    if d0 > 0. && d1 <= 0. {
                        let t = d0 / (d0 - d1);
                        if w.trace_height(
                            self.position,
                            self.position + self.velocity * remaining,
                            self.height(),
                        )
                        .0 + 0.00001
                            < t
                        {
                            continue;
                        }
                        let at = center.lerp(end, t);
                        let mapped = src.map_point(dst, at) + dst.normal * 0.05;
                        if src.fits(at, extent)
                            && dst.fits(mapped, extent)
                            && !w.overlaps(mapped - offset, self.height())
                        {
                            self.position = mapped - offset;
                            self.velocity = src.map_vector(dst, self.velocity);
                            let forward = src.map_vector(dst, self.forward());
                            self.yaw = forward.y.atan2(forward.x);
                            self.pitch = forward.z.clamp(-1., 1.).asin();
                            self.view_up = src.map_vector(dst, self.view_up);
                            self.grounded = false;
                            self.portal_crossings += 1;
                            self.last_portal_transfer = Some((src, dst));
                            remaining *= 1. - t;
                            crossed = true;
                            break;
                        } else if d0 < src.normal.abs().dot(extent) + 2.
                            && (at - src.center).dot(src.right()).abs()
                                < crate::portal::HALF_WIDTH + extent.length()
                            && (at - src.center).dot(src.up).abs()
                                < crate::portal::HALF_HEIGHT + extent.length()
                        {
                            // A partially inserted hull can cease fitting while it jumps.
                            // The convex sweep starts inside the backing brush and cannot
                            // supply an entry hit: retain a front-plane barrier instead of
                            // allowing motion through the wall without a valid transfer.
                            self.position = at + src.normal * 0.03125 - offset;
                            let into = self.velocity.dot(src.normal);
                            if into < 0. {
                                self.velocity -= src.normal * into;
                            }
                            remaining *= 1. - t;
                            crossed = true;
                            break;
                        }
                    }
                }
                if crossed {
                    continue;
                }
            }
            let (t, n) = w.trace_height(
                self.position,
                self.position + self.velocity * remaining,
                self.height(),
            );
            self.position += self.velocity * remaining * t;
            if t >= 1. {
                break;
            }
            remaining *= 1. - t;
            let into = self.velocity.dot(n);
            if into < 0. {
                self.velocity -= n * into;
            }
            if n.z > 0.7 {
                self.grounded = true;
            }
        }
        // A raised one-way mesh floor need not block horizontal travel: the
        // leading hull can already overlap it below its top. Still negotiate
        // the ordinary 18-unit step, and allow an equal-distance raised route.
        // Otherwise we leave the rim grounded below the cabin floor and fall
        // through it; centre-staged elevator tests cannot catch this hand-off.
        if was_grounded
            && !jump
            && (initial_velocity * dt).truncate().length_squared() > 0.001
        {
            let (up, _) = w.trace_height(start, start + Vec3::Z * 18., self.height());
            if up >= 1. {
                let raised = start + Vec3::Z * 18.;
                let horizontal = Vec3::new(initial_velocity.x, initial_velocity.y, 0.) * dt;
                let (along, _) = w.trace_height(raised, raised + horizontal, self.height());
                let top = raised + horizontal * along;
                let (down, n) = w.trace_height(top, top - Vec3::Z * 20., self.height());
                let stepped = top - Vec3::Z * 20. * down;
                if down < 1.
                    && n.z > 0.7
                    && ((stepped - start).truncate().length_squared()
                        > (self.position - start).truncate().length_squared()
                        || ((stepped - self.position).truncate().length_squared() < 0.001
                            && stepped.z > start.z + 0.001))
                    && !w.overlaps(stepped, self.height())
                {
                    self.position = stepped;
                    self.velocity = Vec3::new(initial_velocity.x, initial_velocity.y, 0.);
                    self.grounded = true;
                }
            }
        }
    }
}
