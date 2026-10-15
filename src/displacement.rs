//! Source BSP v20 DISPINFO v0 (176), DISP_VERTS v0 (20), DISP_TRIS v0 (2).
//! Full-resolution checkerboard tessellation; no neighbor stitching/LOD or blend alpha.
use glam::Vec3;
#[derive(Clone, Copy, Debug)]
pub struct TerrainVertex {
    pub position: Vec3,
    pub base: Vec3,
    pub normal: Vec3,
}
fn int(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn float(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
pub fn tessellate(
    corners: &[Vec3],
    info: &[u8],
    verts: &[u8],
    tris: &[u8],
) -> Result<Vec<TerrainVertex>, String> {
    if corners.len() != 4 || info.len() != 176 {
        return Err("displacement requires quad/176-byte info".into());
    }
    let start = Vec3::new(float(info, 0), float(info, 4), float(info, 8));
    let corner = corners
        .iter()
        .position(|p| p.distance_squared(start) < 0.01)
        .ok_or("displacement start corner mismatch")?;
    let c: [Vec3; 4] = std::array::from_fn(|n| corners[(corner + n) % 4]);
    let power = int(info, 20);
    if !(2..=4).contains(&power) {
        return Err("unsupported displacement power".into());
    }
    let side = 1usize << power;
    let width = side + 1;
    let vs = int(info, 12);
    let ts = int(info, 16);
    if vs < 0 || ts < 0 {
        return Err("negative displacement start".into());
    }
    let data = verts
        .get(vs as usize * 20..(vs as usize + width * width) * 20)
        .ok_or("displacement vertex bounds")?;
    tris.get(ts as usize * 2..(ts as usize + side * side * 2) * 2)
        .ok_or("displacement triangle bounds")?;
    let elevation = float(info, 24);
    let outward = -(c[1] - c[0]).cross(c[3] - c[0]).normalize_or_zero();
    if !elevation.is_finite() || outward.length_squared() < 0.5 {
        return Err("invalid displacement elevation/quad".into());
    }
    let mut grid = Vec::with_capacity(width * width);
    for y in 0..width {
        for x in 0..width {
            let base = c[0].lerp(c[1], y as f32 / side as f32).lerp(
                c[3].lerp(c[2], y as f32 / side as f32),
                x as f32 / side as f32,
            );
            let a = &data[(y * width + x) * 20..];
            let direction = Vec3::new(float(a, 0), float(a, 4), float(a, 8));
            let distance = float(a, 12);
            let position = base + direction * distance + outward * elevation;
            if !base.is_finite() || !position.is_finite() || !float(a, 16).is_finite() {
                return Err("nonfinite displacement vertex".into());
            }
            grid.push(TerrainVertex {
                position,
                base,
                normal: Vec3::ZERO,
            });
        }
    }
    let mut indices = Vec::with_capacity(side * side * 6);
    for y in 0..side {
        for x in 0..side {
            let a = y * width + x;
            let b = a + 1;
            let d = a + width;
            let c = d + 1;
            if (x + y) % 2 == 0 {
                indices.extend([a, d, c, a, c, b]);
            } else {
                indices.extend([a, d, b, b, d, c]);
            }
        }
    }
    for t in indices.chunks_exact(3) {
        let n = (grid[t[1]].position - grid[t[0]].position)
            .cross(grid[t[2]].position - grid[t[0]].position);
        for &j in t {
            grid[j].normal -= n;
        }
    }
    for v in &mut grid {
        v.normal = v.normal.normalize_or_zero();
    }
    Ok(indices.into_iter().map(|j| grid[j]).collect())
}
