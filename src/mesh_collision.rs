//! Continuous AABB/triangle SAT, without extruding triangles into infinite prisms.
use glam::Vec3;
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
        let delta = end - start;
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
        let mut enter = f32::NEG_INFINITY;
        let mut exit = f32::INFINITY;
        let mut normal = Vec3::ZERO;
        for axis in axes {
            if axis.length_squared() < 1e-10 {
                continue;
            }
            let axis = axis.normalize();
            let r = axis.abs().dot(extent);
            let p = self.vertices.map(|v| axis.dot(v));
            let lo = p[0].min(p[1]).min(p[2]) - r;
            let hi = p[0].max(p[1]).max(p[2]) + r;
            let c = axis.dot(start);
            let d = axis.dot(delta);
            if d.abs() < 1e-8 {
                if c < lo || c > hi {
                    return (1., Vec3::ZERO);
                }
                continue;
            }
            let (a, b, n) = if d > 0. {
                ((lo - c) / d, (hi - c) / d, -axis)
            } else {
                ((hi - c) / d, (lo - c) / d, axis)
            };
            if a > enter {
                enter = a;
                normal = n;
            }
            exit = exit.min(b);
            if enter > exit {
                return (1., Vec3::ZERO);
            }
        }
        // Existing intersections are not entry hits. Keep the skin on a genuine
        // entry only, so touching/tangential or departing triangles do not snag.
        if enter < -1e-5 || enter > 1. || exit < 0. || normal.dot(delta) >= -1e-8 {
            return (1., Vec3::ZERO);
        }
        ((enter - 0.03125 / normal.dot(delta).abs()).max(0.), normal)
    }
}
