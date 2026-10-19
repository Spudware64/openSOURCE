//! Conservative, order-preserving submission ranges over the shared vertex buffer.
use crate::scene::MeshVertex;
use glam::{Mat4, Vec3, Vec4};
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub(super) struct Bounds {
    min: Vec3,
    max: Vec3,
}
impl Bounds {
    pub(super) fn visible(self, matrix: Mat4, plane: Vec4) -> bool {
        let corners: [Vec4; 8] = std::array::from_fn(|i| {
            Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            )
            .extend(1.)
        });
        // Strict rejection with a world-space margin: crossing/touching bounds stay.
        if corners.iter().all(|p| plane.dot(*p) < -0.01) {
            return false;
        }
        let clip = corners.map(|p| matrix * p);
        !(0..6).any(|n| {
            clip.iter().all(|p| {
                let d = match n {
                    0 => p.x + p.w,
                    1 => p.w - p.x,
                    2 => p.y + p.w,
                    3 => p.w - p.y,
                    4 => p.z,
                    _ => p.w - p.z,
                };
                d < -0.0001 * p.w.abs().max(1.)
            })
        })
    }
}

pub(super) struct Chunk {
    range: Range<u32>,
    bounds: Option<Bounds>,
}
pub(super) fn static_bounds(
    vertices: &[MeshVertex],
    range: Range<usize>,
    animated: &[Range<usize>],
) -> Option<Bounds> {
    let vs = &vertices[range.clone()];
    if vs.iter().any(|v| v.model != 0)
        || animated
            .iter()
            .any(|r| r.start < range.end && r.end > range.start)
    {
        return None;
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for v in vs {
        let p = Vec3::from(v.position);
        if !p.is_finite() {
            return None;
        }
        min = min.min(p);
        max = max.max(p);
    }
    Some(Bounds { min, max })
}
pub(super) fn build(vertices: &[MeshVertex], animated: &[Range<usize>], size: usize) -> Vec<Chunk> {
    assert!(size > 0 && size.is_multiple_of(3));
    vertices
        .chunks(size)
        .enumerate()
        .map(|(i, vs)| {
            let start = i * size;
            let end = start + vs.len();
            Chunk {
                range: start as u32..end as u32,
                bounds: static_bounds(vertices, start..end, animated),
            }
        })
        .collect()
}
pub(super) fn ranges(chunks: &[Chunk], matrix: Mat4, plane: Vec4) -> Vec<Range<u32>> {
    let mut result: Vec<Range<u32>> = Vec::new();
    for c in chunks {
        if c.bounds.is_some_and(|b| !b.visible(matrix, plane)) {
            continue;
        }
        if let Some(last) = result.last_mut() {
            if last.end == c.range.start {
                last.end = c.range.end;
                continue;
            }
        }
        result.push(c.range.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_and_animated_chunks_are_never_rejected_and_ranges_merge() {
        use bytemuck::Zeroable;
        let mut vertices = vec![MeshVertex::zeroed(); 18];
        for v in &mut vertices {
            v.position = [3., 0., 0.5];
        }
        for v in &mut vertices[6..9] {
            v.model = 1;
        }
        let chunks = build(&vertices, &[12..15], 3);
        assert_eq!(
            ranges(&chunks, Mat4::IDENTITY, Vec4::ZERO),
            vec![6..9, 12..15]
        );
        assert_eq!(
            ranges(&chunks, Mat4::from_scale(Vec3::splat(0.1)), Vec4::ZERO),
            vec![0..18]
        );
    }
    #[test]
    fn static_triangle_bounds_keep_model_and_animation_ranges_live() {
        use bytemuck::Zeroable;
        let mut vertices = vec![MeshVertex::zeroed(); 9];
        for v in &mut vertices {
            v.position = [4., 0., 0.5];
        }
        vertices[3].model = 7;
        assert!(!static_bounds(&vertices, 0..3, &[6..9])
            .unwrap()
            .visible(Mat4::IDENTITY, Vec4::ZERO));
        assert!(static_bounds(&vertices, 3..6, &[6..9]).is_none());
        assert!(static_bounds(&vertices, 6..9, &[6..9]).is_none());
    }
    #[test]
    fn rejects_only_wholly_outside_bounds() {
        let b = Bounds {
            min: Vec3::new(2., 0., 0.2),
            max: Vec3::new(3., 1., 0.8),
        };
        assert!(!b.visible(Mat4::IDENTITY, Vec4::ZERO));
        let crossing = Bounds {
            min: Vec3::new(-2., -2., -0.2),
            max: Vec3::new(2., 2., 0.8),
        };
        assert!(crossing.visible(Mat4::IDENTITY, Vec4::ZERO));
        assert!(crossing.visible(Mat4::IDENTITY, Vec4::new(1., 0., 0., 0.)));
        assert!(!b.visible(Mat4::IDENTITY, Vec4::new(-1., 0., 0., 1.)));
    }
}
