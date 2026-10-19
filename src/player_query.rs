//! Configuration-space queries for Source's upright box. Parry ray/triangle
//! narrowphase avoids metre-space GJK's sub-skin tolerance on large BSP hulls.
//! Geometry is expanded by the exact Source support function, not a capsule.
use crate::play::Plane;
use glam::Vec3;
use rapier3d::math::Vector;
use rapier3d::parry::{query::Ray, shape::SharedShape};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};
const SCALE: f32 = 0.0254;
thread_local! {
    static QUERIES: Cell<u64> = const { Cell::new(0) };
    static CACHE: RefCell<HashMap<Vec<u32>, Option<(SharedShape, Vec3)>>> = RefCell::new(HashMap::new());
}
pub fn queries() -> u64 {
    QUERIES.with(Cell::get)
}
fn build(planes: &[Plane], extent: Vec3) -> Option<(SharedShape, Vec3)> {
    let _profile = crate::frame_profile::scope(18);
    crate::frame_profile::count(14, 1);
    if let [p] = planes {
        return Some((
            SharedShape::halfspace(Vector::new(p.normal.x, p.normal.y, p.normal.z)),
            p.normal * (p.distance + p.normal.abs().dot(extent)),
        ));
    }
    let planes: Vec<_> = planes
        .iter()
        .map(|p| {
            (
                p.normal.as_dvec3(),
                (p.distance as f64) + p.normal.abs().as_dvec3().dot(extent.as_dvec3()),
            )
        })
        .collect();
    let mut points = Vec::new();
    for i in 0..planes.len() {
        for j in i + 1..planes.len() {
            for k in j + 1..planes.len() {
                let (a, b, c) = (planes[i], planes[j], planes[k]);
                let det = a.0.dot(b.0.cross(c.0));
                if det.abs() < 1e-10 {
                    continue;
                }
                let v = (a.1 * b.0.cross(c.0) + b.1 * c.0.cross(a.0) + c.1 * a.0.cross(b.0)) / det;
                if v.is_finite() && planes.iter().all(|p| p.0.dot(v) - p.1 <= 1e-6) {
                    points.push(v);
                }
            }
        }
    }
    if points.is_empty() {
        return None;
    }
    let center = (points.iter().copied().sum::<glam::DVec3>() / points.len() as f64).as_vec3();
    let points: Vec<_> = points
        .iter()
        .map(|v| {
            let p = ((*v - center.as_dvec3()) * SCALE as f64).as_vec3();
            Vector::new(p.x, p.y, p.z)
        })
        .collect();
    let hull = SharedShape::convex_hull(&points)?;
    let (vertices, indices) = hull.as_convex_polyhedron()?.to_trimesh();
    Some((SharedShape::trimesh(vertices, indices).ok()?, center))
}
pub fn cast(planes: &[Plane], start: Vec3, end: Vec3, extent: Vec3) -> (f32, Vec3) {
    let delta = end - start;
    if delta.length_squared() == 0. {
        return (1., Vec3::ZERO);
    }
    // Conservative halfspace broad rejection before cache lookup/hull building.
    // This only rejects a separating plane; all accepted TOIs come from Parry.
    if planes.iter().any(|p| {
        let a = p.normal.dot(start) - p.distance - p.normal.abs().dot(extent);
        a > 0.03125 && p.normal.dot(end - start) >= 0.
    }) {
        return (1., Vec3::ZERO);
    }
    let mut key = Vec::with_capacity(planes.len() * 4 + 3);
    key.extend(extent.to_array().map(f32::to_bits));
    for p in planes {
        key.extend(p.normal.to_array().map(f32::to_bits));
        key.push(p.distance.to_bits());
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 16384 {
            crate::frame_profile::count(15, 1);
            cache.clear();
        }
        let Some((shape, center)) = cache
            .entry(key)
            .or_insert_with(|| build(planes, extent))
            .as_ref()
        else {
            return (1., Vec3::ZERO);
        };
        let a = (start - *center) * SCALE;
        let d = delta * SCALE;
        let ray = Ray::new(Vector::new(a.x, a.y, a.z), Vector::new(d.x, d.y, d.z));
        // Reject existing penetration and tangency: Source's controller handles
        // recovery separately; an exit face is not a new blocking entry.
        if planes
            .iter()
            .all(|p| p.normal.dot(start) - p.distance - p.normal.abs().dot(extent) <= 0.)
        {
            return (1., Vec3::ZERO);
        }
        QUERIES.with(|n| n.set(n.get() + 1));
        let Some(h) = shape.cast_local_ray_and_get_normal(&ray, f32::MAX, true) else {
            return (1., Vec3::ZERO);
        };
        let mut n = Vec3::new(h.normal.x, h.normal.y, h.normal.z);
        // Retain authored normals, rather than hull triangulation round-off.
        if let Some(p) = planes
            .iter()
            .max_by(|a, b| a.normal.dot(n).total_cmp(&b.normal.dot(n)))
        {
            if p.normal.dot(n) > 0.9999 {
                n = p.normal;
            }
        }
        let speed = -n.dot(delta);
        if speed <= 1e-8 {
            return (1., Vec3::ZERO);
        }
        ((h.time_of_impact - 0.03125 / speed).max(0.), n)
    })
}
