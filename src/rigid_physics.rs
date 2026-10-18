//! First runtime migration: free, upright cubes use Rapier contacts and impulses.
//! Source coordinates are Z-up; 1 Source unit is 0.0254 metres.
use crate::play::{Plane, World};
use glam::Vec3;
use rapier3d::prelude::*;
use std::collections::HashMap;
const SCALE: f32 = 0.0254;
fn vector(v: Vec3) -> Vector {
    Vector::new(v.x, v.y, v.z) * SCALE
}
fn source(v: Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z) / SCALE
}
fn same_planes(a: &[Plane], b: &[Plane]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.normal == b.normal && a.distance == b.distance)
}

// Only accept a common translation when all ordered plane normals are identical.
// Recheck every plane so deformation cannot accidentally reuse a stale hull.
fn translation_between(old: &[Plane], new: &[Plane]) -> Option<Vec3> {
    if old.len() != new.len()
        || old.is_empty()
        || old.iter().zip(new).any(|(a, b)| a.normal != b.normal)
    {
        return None;
    }
    if let [a] = old {
        return Some(a.normal * (new[0].distance - a.distance));
    }
    for j in 1..old.len() {
        for k in j + 1..old.len() {
            let (a, b, c) = (old[0].normal, old[j].normal, old[k].normal);
            let det = a.dot(b.cross(c));
            if det.abs() < 1e-6 {
                continue;
            }
            let delta = ((new[0].distance - old[0].distance) * b.cross(c)
                + (new[j].distance - old[j].distance) * c.cross(a)
                + (new[k].distance - old[k].distance) * a.cross(b))
                / det;
            return (delta.is_finite()
                && old.iter().zip(new).all(|(a, b)| {
                    (a.normal.dot(delta) - (b.distance - a.distance)).abs() <= 1e-5
                }))
            .then_some(delta);
        }
    }
    None
}

#[derive(Default)]
pub(crate) struct RigidPhysics {
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad: BroadPhaseBvh,
    narrow: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: ImpulseJointSet,
    multibody: MultibodyJointSet,
    soft: SoftBodySet,
    ccd: CCDSolver,
    handles: HashMap<usize, RigidBodyHandle>,
    geometry: Vec<Vec<Plane>>,
    scenery: Vec<Option<ColliderHandle>>,
    meshes: Vec<(usize, ColliderHandle)>,
    #[cfg(test)]
    hull_builds: usize,
    accumulator: f32,
}
impl RigidPhysics {
    // One immutable posed triangle mesh per rigid, single-clip MDL instance.
    // Parent motion updates its collider transform, not BSP hulls or mesh data.
    pub fn bind_scene(&mut self, scene: &crate::scene::Scene, dynamic_models: &[usize]) {
        for (_, h) in self.meshes.drain(..) {
            self.colliders
                .remove(h, &mut self.islands, &mut self.bodies, &mut self.soft, true);
        }
        for instance in &scene.animations {
            if !instance.solid || instance.skeleton.clips.len() != 1 {
                continue;
            }
            let vertices = &scene.vertices[instance.range.clone()];
            if vertices.is_empty() || dynamic_models.contains(&(vertices[0].model as usize)) {
                continue;
            }
            let points = vertices
                .iter()
                .map(|v| vector(Vec3::from(v.position)))
                .collect();
            let triangles = (0..vertices.len() as u32 / 3)
                .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                .collect();
            if let Ok(builder) = ColliderBuilder::trimesh(points, triangles) {
                let model = vertices[0].model as usize;
                let h = self.colliders.insert(
                    builder
                        .translation(vector(scene.origins[model]))
                        .friction(0.6)
                        .build(),
                );
                self.meshes.push((model, h));
            }
        }
    }
    pub fn sync_mesh_poses(
        &mut self,
        origins: &[Vec3],
        rotations: &[glam::Mat3],
        visible: &[bool],
    ) {
        for &(model, h) in &self.meshes {
            if model >= origins.len() {
                continue;
            }
            let c = &mut self.colliders[h];
            c.set_enabled(visible.get(model).copied().unwrap_or(false));
            let q = glam::Quat::from_mat3(&rotations[model]);
            c.set_position(Pose::from_parts(
                vector(origins[model]),
                Rotation::from_xyzw(q.x, q.y, q.z, q.w),
            ));
        }
    }

    pub fn sync_world(&mut self, brushes: &[Vec<Plane>]) {
        // Brush slots are the World's stable portal/contact identifiers. Keep
        // unchanged shapes and handles; never clone or rebuild the whole scene.
        while self.scenery.len() > brushes.len() {
            if let Some(h) = self.scenery.pop().unwrap() {
                self.colliders
                    .remove(h, &mut self.islands, &mut self.bodies, &mut self.soft, true);
            }
        }
        self.geometry.truncate(brushes.len());
        for (index, brush) in brushes.iter().enumerate() {
            if self
                .geometry
                .get(index)
                .is_some_and(|old| same_planes(old, brush))
            {
                continue;
            }
            if index == self.scenery.len() {
                self.scenery.push(None);
                self.geometry.push(Vec::new());
            }
            let _profile = crate::frame_profile::scope(12);
            // A translation reuses the immutable hull. Rotation/topology changes
            // fall back to rebuilding only this brush, not unrelated scenery.
            if let Some(delta) = translation_between(&self.geometry[index], brush) {
                if let Some(h) = self.scenery[index] {
                    let c = &mut self.colliders[h];
                    c.set_translation(c.translation() + vector(delta));
                    self.geometry[index].clone_from(brush);
                    continue;
                }
            }
            crate::frame_profile::count(6, 1);
            #[cfg(test)]
            {
                self.hull_builds += 1;
            }
            self.geometry[index].clone_from(brush);
            if let Some(h) = self.scenery[index].take() {
                self.colliders
                    .remove(h, &mut self.islands, &mut self.bodies, &mut self.soft, true);
            }
            if let [plane] = brush.as_slice() {
                {
                    let shape = ColliderBuilder::halfspace(rapier3d::na::Unit::new_unchecked(
                        Vector::new(plane.normal.x, plane.normal.y, plane.normal.z),
                    ));
                    self.scenery[index] = Some(
                        self.colliders.insert(
                            shape
                                .user_data((index + 1) as u128)
                                .translation(vector(plane.normal * plane.distance))
                                .build(),
                        ),
                    );
                }
                continue;
            }
            let mut points = Vec::new();
            // Intersect BSP halfspaces, retaining only vertices inside the convex solid.
            for i in 0..brush.len() {
                for j in i + 1..brush.len() {
                    for k in j + 1..brush.len() {
                        let (a, b, c) = (brush[i], brush[j], brush[k]);
                        let det = a.normal.dot(b.normal.cross(c.normal));
                        if det.abs() < 1e-6 {
                            continue;
                        }
                        let p = (a.distance * b.normal.cross(c.normal)
                            + b.distance * c.normal.cross(a.normal)
                            + c.distance * a.normal.cross(b.normal))
                            / det;
                        if p.is_finite()
                            && brush.iter().all(|s| s.normal.dot(p) - s.distance <= 0.02)
                        {
                            points.push(vector(p));
                        }
                    }
                }
            }
            if let Some(shape) = ColliderBuilder::convex_hull(&points) {
                self.scenery[index] = Some(
                    self.colliders
                        .insert(shape.user_data((index + 1) as u128).friction(0.6).build()),
                );
            }
        }
    }
    pub fn retain(&mut self, ids: &[usize]) {
        let removed: Vec<_> = self
            .handles
            .keys()
            .copied()
            .filter(|id| !ids.contains(id))
            .collect();
        for id in removed {
            let h = self.handles.remove(&id).unwrap();
            self.bodies.remove(
                h,
                &mut self.islands,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multibody,
                &mut self.soft,
                true,
            );
        }
    }
    pub fn sync_body(&mut self, id: usize, mut center: Vec3, extent: Vec3, velocity: Vec3) {
        // Authored/render-derived bounds may start a fraction inside a floor.
        // Recover that spawn/teleport overlap before the first contact solver
        // can remove approach momentum against a temporarily rejected aperture.
        let external_move = self
            .handles
            .get(&id)
            .is_none_or(|h| source(self.bodies[*h].translation()).distance_squared(center) > 0.01);
        if external_move {
            let e = vector(extent);
            let shape = Cuboid::new(e);
            let raised = Pose::from_translation(vector(center + Vec3::Z));
            let mut fraction = 1.;
            for h in self.scenery.iter().flatten() {
                let c = &self.colliders[*h];
                if let Ok(Some(hit)) = rapier3d::parry::query::cast_shapes(
                    &raised,
                    vector(-Vec3::Z),
                    &shape,
                    c.position(),
                    Vector::ZERO,
                    c.shape(),
                    rapier3d::parry::query::ShapeCastOptions {
                        max_time_of_impact: 1.,
                        target_distance: 0.03125 * SCALE,
                        ..Default::default()
                    },
                ) {
                    if hit.normal2.z > 0.7 && hit.time_of_impact > 0. {
                        fraction = fraction.min(hit.time_of_impact);
                    }
                }
            }
            center += Vec3::Z * (1. - fraction);
        }
        let h = *self.handles.entry(id).or_insert_with(|| {
            let h = self.bodies.insert(
                RigidBodyBuilder::dynamic()
                    .translation(vector(center))
                    .user_data((id + 1) as u128)
                    .lock_rotations()
                    .ccd_enabled(true)
                    .build(),
            );
            let e = vector(extent.max(Vec3::splat(0.01)));
            self.colliders.insert_with_parent(
                ColliderBuilder::cuboid(e.x, e.y, e.z)
                    .active_hooks(ActiveHooks::MODIFY_SOLVER_CONTACTS)
                    .contact_skin(0.03125 * SCALE)
                    .friction(0.6)
                    .restitution(0.)
                    .build(),
                h,
                &mut self.bodies,
            );
            h
        });
        let body = &mut self.bodies[h];
        if source(body.translation()).distance_squared(center) > 0.000001 {
            body.set_translation(vector(center), true);
        }
        if source(body.linvel()).distance_squared(velocity) > 0.000001 {
            body.set_linvel(vector(velocity), true);
        }
    }
    pub fn step(&mut self, dt: f32, world: &World) {
        self.accumulator += dt;
        let parameters = IntegrationParameters {
            dt: 1. / 120.,
            ..Default::default()
        };
        while self.accumulator + 1e-7 >= parameters.dt {
            let hooks = PortalContacts {
                world,
                dt: parameters.dt,
            };
            let mut previous = Vec::new();
            for (&id, &handle) in &self.handles {
                let body = &mut self.bodies[handle];
                let start = source(body.translation());
                let extent = source(
                    self.colliders[body.colliders()[0]]
                        .shape()
                        .as_cuboid()
                        .unwrap()
                        .half_extents,
                );
                let end = start + source(body.linvel()) * parameters.dt;
                let opening = world
                    .brushes
                    .iter()
                    .any(|b| world.aperture_except(b, start, end, extent, Some(id)));
                body.enable_ccd(!opening);
                previous.push((id, handle, start, extent));
            }
            self.pipeline.step(
                vector(Vec3::new(0., 0., -600.)),
                &parameters,
                &mut self.islands,
                &mut self.broad,
                &mut self.narrow,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multibody,
                &mut self.soft,
                &mut self.ccd,
                &hooks,
                &(),
            );
            if let [Some(a), Some(b)] = world.portals {
                for (id, handle, start, extent) in previous {
                    let body = &mut self.bodies[handle];
                    let end = source(body.translation());
                    for (src, dst) in [(a, b), (b, a)] {
                        let d0 = src.normal.dot(start - src.center);
                        let d1 = src.normal.dot(end - src.center);
                        if d0 > 0. && d1 <= 0. {
                            let t = d0 / (d0 - d1);
                            let at = start.lerp(end, t);
                            let mapped = src.map_point(dst, at) + dst.normal * 0.05;
                            if src.fits(at, extent)
                                && dst.fits(mapped, extent)
                                && !world.body_overlap(id, mapped, extent)
                            {
                                body.set_translation(
                                    vector(src.map_point(dst, end) + dst.normal * 0.05),
                                    true,
                                );
                                body.set_linvel(
                                    vector(src.map_vector(dst, source(body.linvel()))),
                                    true,
                                );
                                break;
                            }
                        }
                    }
                }
            }
            self.accumulator -= parameters.dt;
        }
    }
    /// Sweep the held cuboid against the same Rapier shapes as free bodies.
    /// Keep aperture eligibility shared with contact hooks, but never discard
    /// a non-backing-plane normal merely because its brush hosts a portal.
    pub fn cast_carry(
        &self,
        id: usize,
        start: Vec3,
        end: Vec3,
        extent: Vec3,
        world: &World,
    ) -> (f32, Vec3) {
        let shape = Cuboid::new(vector(extent));
        let pose = Pose::from_translation(vector(start));
        let movement = vector(end - start);
        let mut result = world.carry_portal_rim(start, end, extent);
        for (_, collider) in self.colliders.iter() {
            if collider
                .parent()
                .is_some_and(|h| self.bodies[h].user_data == (id + 1) as u128)
            {
                continue;
            }
            let Ok(Some(hit)) = rapier3d::parry::query::cast_shapes(
                &pose,
                movement,
                &shape,
                collider.position(),
                Vector::ZERO,
                collider.shape(),
                rapier3d::parry::query::ShapeCastOptions {
                    max_time_of_impact: result.0,
                    target_distance: 0.03125 * SCALE,
                    stop_at_penetration: false,
                    ..Default::default()
                },
            ) else {
                continue;
            };
            let n = Vec3::new(hit.normal2.x, hit.normal2.y, hit.normal2.z);
            let brush = (collider.parent().is_none() && collider.user_data > 0)
                .then(|| world.brushes.get(collider.user_data as usize - 1))
                .flatten();
            if brush.is_some_and(|brush| world.aperture_except(brush, start, end, extent, Some(id)))
                && world
                    .portals
                    .iter()
                    .flatten()
                    .any(|p| n.dot(p.normal).abs() > 0.999)
            {
                continue;
            }
            if hit.time_of_impact < result.0 {
                result = (hit.time_of_impact, n);
            }
        }
        result
    }

    pub fn collider_center(&self, id: usize) -> Option<Vec3> {
        let body = self.bodies.get(*self.handles.get(&id)?)?;
        let collider = self.colliders.get(*body.colliders().first()?)?;
        Some(source(collider.translation()))
    }

    pub fn state(&self, id: usize) -> (Vec3, Vec3) {
        let b = &self.bodies[self.handles[&id]];
        (source(b.translation()), source(b.linvel()))
    }
}

#[cfg(test)]
mod carry_tests {
    use super::*;
    fn box_brush(center: Vec3) -> Vec<Plane> {
        [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
            .into_iter()
            .map(|normal| Plane {
                normal,
                distance: normal.dot(center) + 8.,
            })
            .collect()
    }

    #[test]
    fn animated_brush_sync_preserves_static_colliders() {
        let mut brushes: Vec<_> = (0..256)
            .map(|i| box_brush(Vec3::new(i as f32 * 32., 100., 0.)))
            .collect();
        brushes.push(box_brush(Vec3::ZERO));
        let mut physics = RigidPhysics::default();
        physics.sync_world(&brushes);
        let static_handle = physics.scenery[0].unwrap();
        let initial_builds = physics.hull_builds;
        let mut peak = std::time::Duration::ZERO;
        let mut total = std::time::Duration::ZERO;
        for tick in 1..=120 {
            brushes[256] = box_brush(Vec3::X * tick as f32);
            let start = std::time::Instant::now();
            physics.sync_world(&brushes);
            let elapsed = start.elapsed();
            peak = peak.max(elapsed);
            total += elapsed;
            assert_eq!(physics.colliders.len(), 257);
        }
        eprintln!("animated sync: 256 static + 1 moving, 120 ticks, total={total:?} peak={peak:?}");
        assert!(
            physics.colliders.get(static_handle).is_some(),
            "animation invalidated an unchanged scenery handle"
        );
        assert_eq!(
            physics.hull_builds, initial_builds,
            "translation must reuse cached hulls"
        );
        let world = World::from_brushes(brushes);
        let (fraction, normal) =
            physics.cast_carry(999, Vec3::ZERO, Vec3::X * 200., Vec3::ONE, &world);
        assert!(
            (fraction - 0.55484).abs() < 0.002,
            "stale moving collider: {fraction}"
        );
        assert!(normal.x < -0.99);
    }

    #[test]
    fn changed_brush_rebuild_removal_and_reinsertion_are_local() {
        let mut brushes = vec![box_brush(Vec3::Y * 100.), box_brush(Vec3::X * 100.)];
        let mut physics = RigidPhysics::default();
        physics.sync_world(&brushes);
        let static_handle = physics.scenery[0].unwrap();
        let rotation = glam::Quat::from_rotation_z(0.4);
        for plane in &mut brushes[1] {
            plane.normal = rotation * plane.normal;
        }
        physics.sync_world(&brushes);
        assert_eq!(physics.hull_builds, 3);
        assert_eq!(physics.scenery[0], Some(static_handle));
        assert_eq!(physics.colliders.len(), 2);
        let direction = rotation * Vec3::X;
        let world = World::from_brushes(brushes.clone());
        let (fraction, normal) =
            physics.cast_carry(999, Vec3::ZERO, direction * 200., Vec3::ONE, &world);
        assert!(fraction > 0.44 && fraction < 0.46, "{fraction}");
        assert!(normal.dot(-direction) > 0.99, "{normal:?}");
        // Invalid geometry must remove its old collider, retaining the slot.
        brushes[1].clear();
        physics.sync_world(&brushes);
        assert_eq!(physics.colliders.len(), 1);
        assert!(physics.scenery[1].is_none());
        brushes[1] = box_brush(Vec3::X * 150.);
        physics.sync_world(&brushes);
        assert_eq!(physics.colliders.len(), 2);
        brushes.truncate(1);
        physics.sync_world(&brushes);
        assert_eq!(physics.colliders.len(), 1);
        assert_eq!(physics.scenery[0], Some(static_handle));
        brushes.push(box_brush(Vec3::X * 200.));
        physics.sync_world(&brushes);
        assert_eq!(physics.colliders.len(), 2);
        let builds = physics.hull_builds;
        physics.sync_world(&brushes);
        assert_eq!(physics.hull_builds, builds);
    }

    #[test]
    fn carry_cast_uses_scene_and_dynamic_colliders() {
        let world = World::from_brushes(vec![vec![Plane {
            normal: -Vec3::X,
            distance: -100.,
        }]]);
        let mut physics = RigidPhysics::default();
        physics.sync_world(&world.brushes);
        let (fraction, normal) =
            physics.cast_carry(0, Vec3::ZERO, Vec3::X * 200., Vec3::splat(16.), &world);
        assert!((fraction - 0.41984).abs() < 0.001, "{fraction}");
        assert!(normal.x < -0.99);
        physics.sync_body(1, Vec3::X * 60., Vec3::splat(16.), Vec3::ZERO);
        let (fraction, _) =
            physics.cast_carry(0, Vec3::ZERO, Vec3::X * 200., Vec3::splat(16.), &world);
        assert!(fraction < 0.15, "dynamic body must block carry: {fraction}");
    }
}

struct PortalContacts<'a> {
    world: &'a World,
    dt: f32,
}
impl PhysicsHooks for PortalContacts<'_> {
    fn modify_solver_contacts(&self, context: &mut ContactModificationContext) {
        let (body, scenery) = match (context.rigid_body1, context.rigid_body2) {
            (Some(h), None) => (h, context.collider2),
            (None, Some(h)) => (h, context.collider1),
            _ => return,
        };
        let index = context.colliders[scenery].user_data as usize;
        if index == 0 {
            return;
        }
        let Some(brush) = self.world.brushes.get(index - 1) else {
            return;
        };
        let body = &context.bodies[body];
        let id = body.user_data as usize - 1;
        let extent = source(
            context.colliders[body.colliders()[0]]
                .shape()
                .as_cuboid()
                .unwrap()
                .half_extents,
        );
        let start = source(body.translation());
        let end = start + source(body.linvel()) * self.dt;
        if !self
            .world
            .aperture_except(brush, start, end, extent, Some(id))
        {
            return;
        }
        if let Some(rigid) = context.rigid_mut() {
            // Remove only the backing-plane manifold, never floor/edge contacts
            // that happen to belong to the same convex BSP brush.
            let normal = Vec3::new(rigid.normal.x, rigid.normal.y, rigid.normal.z);
            if self
                .world
                .portals
                .iter()
                .flatten()
                .any(|p| normal.dot(p.normal).abs() > 0.999)
            {
                rigid.solver_contacts.clear();
            }
        }
    }
}
