#[cfg(target_arch = "wasm32")]
pub mod browser;
mod rigid_physics;
mod player_query;
pub mod animation;
pub mod audio;
mod mesh_collision;
pub mod portal;
pub mod session;
pub mod setup;
pub fn validate(b: &[u8]) -> Result<(), String> {
    if b.len() < 1036 || &b[..4] != b"VBSP" {
        return Err("invalid VBSP header".into());
    }
    let v = i32::from_le_bytes(b[4..8].try_into().unwrap());
    if !(19..=20).contains(&v) {
        return Err("unsupported BSP version".into());
    }
    Ok(())
}
pub fn lump(b: &[u8], id: usize, stride: usize) -> Result<&[u8], String> {
    validate(b)?;
    if id >= 64 || stride == 0 {
        return Err("invalid lump request".into());
    }
    let h = 8 + id * 16;
    let off = i32::from_le_bytes(b[h..h + 4].try_into().unwrap());
    let len = i32::from_le_bytes(b[h + 4..h + 8].try_into().unwrap());
    if off < 0
        || len < 0
        || !(len as usize).is_multiple_of(stride)
        || (off as usize)
            .checked_add(len as usize)
            .is_none_or(|end| end > b.len())
    {
        return Err(format!("invalid bounds/stride for lump {id}"));
    }
    if b[h + 12..h + 16] != [0; 4] {
        return Err(format!("compressed lump {id} unsupported"));
    }
    Ok(&b[off as usize..off as usize + len as usize])
}
pub fn polygon(v: &[[f32; 3]], e: &[[u16; 2]], s: &[i32]) -> Result<Vec<[f32; 3]>, String> {
    s.iter()
        .map(|&se| {
            let edge = e
                .get(se.unsigned_abs() as usize)
                .ok_or("surfedge out of bounds")?;
            v.get(edge[usize::from(se < 0)] as usize)
                .copied()
                .ok_or("vertex out of bounds".into())
        })
        .collect()
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}
pub fn load(b: &[u8]) -> Result<Vec<Vertex>, String> {
    validate(b)?;
    for (id, expected) in [(3, 0), (7, 1), (12, 0), (13, 0), (14, 0)] {
        let h = 8 + id * 16;
        if i32::from_le_bytes(b[h + 8..h + 12].try_into().unwrap()) != expected {
            return Err(format!("unsupported layout version for lump {id}"));
        }
    }
    let i = |a: &[u8], o: usize| i32::from_le_bytes(a[o..o + 4].try_into().unwrap());
    let u = |a: &[u8], o: usize| u16::from_le_bytes(a[o..o + 2].try_into().unwrap());
    let v: Vec<[f32; 3]> = lump(b, 3, 12)?
        .chunks_exact(12)
        .map(|a| {
            std::array::from_fn(|n| f32::from_le_bytes(a[n * 4..n * 4 + 4].try_into().unwrap()))
        })
        .collect();
    if v.iter().flatten().any(|x| !x.is_finite()) {
        return Err("nonfinite vertex".into());
    }
    let e: Vec<[u16; 2]> = lump(b, 12, 4)?
        .chunks_exact(4)
        .map(|a| [u(a, 0), u(a, 2)])
        .collect();
    let s: Vec<i32> = lump(b, 13, 4)?.chunks_exact(4).map(|a| i(a, 0)).collect();
    polygon(&v, &e, &s)?; // Validate even unreferenced edges.
    let faces = lump(b, 7, 56)?;
    let models = lump(b, 14, 48)?;
    for a in models.chunks_exact(48) {
        let first = i(a, 40);
        let count = i(a, 44);
        if first < 0 || count < 0 || first as usize + count as usize > faces.len() / 56 {
            return Err("model face range".into());
        }
    }
    let mut out = Vec::new();
    for (fi, a) in faces.chunks_exact(56).enumerate() {
        let first = i(a, 4);
        let count = u(a, 8) as usize;
        if first < 0 || first as usize + count > s.len() {
            return Err("face surfedge range".into());
        }
        let p = polygon(&v, &e, &s[first as usize..first as usize + count])?;
        if p.len() < 3 {
            continue;
        }
        let n = (glam::Vec3::from(p[1]) - glam::Vec3::from(p[0]))
            .cross(glam::Vec3::from(p[2]) - glam::Vec3::from(p[0]))
            .normalize_or_zero();
        let shade = 0.35 + 0.65 * n.dot(glam::Vec3::new(0.3, -0.5, 0.8).normalize()).abs();
        let color = [
            (0.45 + (fi % 7) as f32 * 0.035) * shade,
            0.65 * shade,
            0.72 * shade,
        ];
        for k in 1..p.len() - 1 {
            for &position in &[p[0], p[k], p[k + 1]] {
                out.push(Vertex { position, color });
            }
        }
    }
    if out.is_empty() {
        return Err("no polygon geometry".into());
    }
    Ok(out)
}
pub mod assets;
#[doc(hidden)]
pub mod chamber01_route;
pub mod displacement;
pub mod entity;
pub mod kv;
pub mod models;
pub mod play;
pub mod props;
pub mod scene;
pub mod scenes;
pub mod studio;
pub mod texture;
pub fn surface_rgba(bytes: &[u8], bgra: bool) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if bgra {
        for pixel in out.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    out
}
mod controller_trace;
#[doc(hidden)]
pub mod frame_profile;
mod window;
pub use window::{interactive, interactive_game, interactive_portals, interactive_scene};
mod gpu;
pub use gpu::{render, render_portal_scene, render_scene};
mod portal_gpu;
mod scene_gpu;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_edges_select_correct_endpoint() {
        let v = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let e = [[0, 1], [2, 1], [2, 0]];
        assert_eq!(polygon(&v, &e, &[0, -1, 2]).unwrap(), v);
        assert!(polygon(&v, &e, &[99]).is_err());
        assert!(polygon(&v, &e, &[i32::MIN]).is_err());
    }
    #[test]
    fn rejects_bad_header() {
        assert!(validate(&[0; 1036]).is_err());
    }
    #[test]
    fn lump_bounds_and_stride() {
        let mut b = vec![0; 1048];
        b[..4].copy_from_slice(b"VBSP");
        b[4] = 20;
        b[56..60].copy_from_slice(&1036i32.to_le_bytes());
        b[60..64].copy_from_slice(&12i32.to_le_bytes());
        assert_eq!(lump(&b, 3, 12).unwrap().len(), 12);
        b[60] = 13;
        assert!(lump(&b, 3, 12).is_err());
        b[60] = 12;
        b[56..60].copy_from_slice(&1040i32.to_le_bytes());
        assert!(lump(&b, 3, 12).is_err());
    }
}
