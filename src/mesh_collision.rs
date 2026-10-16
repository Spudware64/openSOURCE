//! Configuration-space AABB/triangle queries using Parry narrowphase.
use glam::Vec3;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn point_ray_hits_actual_triangle() {
        let t = Triangle::new([Vec3::ZERO, Vec3::X * 10., Vec3::Y * 10.]);
        let h = t.trace(Vec3::new(2., 2., 5.), Vec3::new(2., 2., -5.), Vec3::ZERO);
        assert!(h.0 < 0.51 && h.0 > 0.49, "{h:?}");
    }
}
#[derive(Clone)]
pub(crate) struct Group {
    triangles: Vec<Triangle>,
    lo: Vec3,
    hi: Vec3,
}
impl Group {
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let lo = triangles
            .iter()
            .fold(Vec3::splat(f32::INFINITY), |a, t| a.min(t.lo));
        let hi = triangles
            .iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), |a, t| a.max(t.hi));
        Self { triangles, lo, hi }
    }
    pub fn trace(&self, start: Vec3, end: Vec3, extent: Vec3) -> (f32, Vec3) {
        if (start.max(end) + extent + Vec3::splat(0.03125))
            .cmplt(self.lo)
            .any()
            || (start.min(end) - extent - Vec3::splat(0.03125))
                .cmpgt(self.hi)
                .any()
        {
            return (1., Vec3::ZERO);
        }
        let mut hit = (1., Vec3::ZERO);
        for triangle in &self.triangles {
            let h = triangle.trace(start, end, extent);
            if h.0 < hit.0 {
                hit = h;
            }
        }
        hit
    }
}
#[derive(Clone)]
pub(crate) struct Triangle {
    vertices: [Vec3; 3],
    lo: Vec3,
    hi: Vec3,
}
impl Triangle {
    pub fn new(vertices: [Vec3; 3]) -> Self {
        Self {
            lo: vertices[0].min(vertices[1]).min(vertices[2]),
            hi: vertices[0].max(vertices[1]).max(vertices[2]),
            vertices,
        }
    }
    pub fn trace(&self, start: Vec3, end: Vec3, extent: Vec3) -> (f32, Vec3) {
        let sweep_lo = start.min(end) - extent - Vec3::splat(0.03125);
        let sweep_hi = start.max(end) + extent + Vec3::splat(0.03125);
        if sweep_hi.cmplt(self.lo).any() || sweep_lo.cmpgt(self.hi).any() {
            return (1., Vec3::ZERO);
        }
        let edges = [
            self.vertices[1] - self.vertices[0],
            self.vertices[2] - self.vertices[1],
            self.vertices[0] - self.vertices[2],
        ];
        let axes = [
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            edges[0].cross(edges[1]),
            edges[0].cross(Vec3::X),
            edges[0].cross(Vec3::Y),
            edges[0].cross(Vec3::Z),
            edges[1].cross(Vec3::X),
            edges[1].cross(Vec3::Y),
            edges[1].cross(Vec3::Z),
            edges[2].cross(Vec3::X),
            edges[2].cross(Vec3::Y),
            edges[2].cross(Vec3::Z),
        ];
        let mut planes = Vec::new();
        for axis in axes {
            if axis.length_squared() < 1e-10 {
                continue;
            }
            let axis = axis.normalize();
            let p = self.vertices.map(|v| axis.dot(v));
            planes.push(crate::play::Plane {
                normal: axis,
                distance: p[0].max(p[1]).max(p[2]),
            });
            planes.push(crate::play::Plane {
                normal: -axis,
                distance: -p[0].min(p[1]).min(p[2]),
            });
        }
        crate::player_query::cast(&planes, start, end, extent)
    }
}
