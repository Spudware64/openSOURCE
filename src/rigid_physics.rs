//! First runtime migration: free, upright cubes use Rapier contacts and impulses.
//! Source coordinates are Z-up; 1 Source unit is 0.0254 metres.
use crate::play::Plane;
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
    scenery: Vec<ColliderHandle>,
    accumulator: f32,
}
impl RigidPhysics {
    pub fn sync_world(&mut self, brushes: &[Vec<Plane>]) {
        let unchanged = self.geometry.len() == brushes.len()
            && self.geometry.iter().zip(brushes).all(|(a, b)| {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.normal == b.normal && a.distance == b.distance)
            });
        if unchanged {
            return;
        }
        for h in self.scenery.drain(..) {
            self.colliders
                .remove(h, &mut self.islands, &mut self.bodies, &mut self.soft, true);
        }
        self.geometry = brushes.to_vec();
        for brush in brushes {
            if let [plane] = brush.as_slice() {
                {
                    let shape = ColliderBuilder::halfspace(rapier3d::na::Unit::new_unchecked(
                        Vector::new(plane.normal.x, plane.normal.y, plane.normal.z),
                    ));
                    self.scenery.push(
                        self.colliders.insert(
                            shape
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
                self.scenery
                    .push(self.colliders.insert(shape.friction(0.6).build()));
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
    pub fn sync_body(&mut self, id: usize, center: Vec3, extent: Vec3, velocity: Vec3) {
        let h = *self.handles.entry(id).or_insert_with(|| {
            let h = self.bodies.insert(
                RigidBodyBuilder::dynamic()
                    .translation(vector(center))
                    .lock_rotations()
                    .ccd_enabled(true)
                    .build(),
            );
            let e = vector(extent.max(Vec3::splat(0.01)));
            self.colliders.insert_with_parent(
                ColliderBuilder::cuboid(e.x, e.y, e.z)
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
    pub fn step(&mut self, dt: f32) {
        self.accumulator += dt;
        let parameters = IntegrationParameters {
            dt: 1. / 120.,
            ..Default::default()
        };
        while self.accumulator + 1e-7 >= parameters.dt {
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
                &(),
                &(),
            );
            self.accumulator -= parameters.dt;
        }
    }
    pub fn state(&self, id: usize) -> (Vec3, Vec3) {
        let b = &self.bodies[self.handles[&id]];
        (source(b.translation()), source(b.linvel()))
    }
}
