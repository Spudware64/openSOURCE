//! Versioned Source 1 BSP model/leaf ownership (not all brushes belong to worldspawn).
use std::collections::BTreeSet;
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> usize {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap()) as usize
}
pub fn model_brush_indices(b: &[u8]) -> Result<Vec<Vec<usize>>, String> {
    crate::validate(b)?;
    let leaf_version = i(b, 8 + 10 * 16 + 8);
    let stride = match leaf_version {
        0 => 56,
        1 => 32,
        _ => return Err("unsupported leaf layout".into()),
    };
    for id in [5, 17, 14] {
        if i(b, 8 + id * 16 + 8) != 0 {
            return Err(format!("unsupported model collision layout {id}"));
        }
    }
    let nodes = crate::lump(b, 5, 32)?;
    let leaves = crate::lump(b, 10, stride)?;
    let refs = crate::lump(b, 17, 2)?;
    let brushes = crate::lump(b, 18, 12)?.len() / 12;
    let models = crate::lump(b, 14, 48)?;
    let mut out = vec![];
    for model in models.chunks_exact(48) {
        let mut stack = vec![i(model, 36)];
        let mut visited = BTreeSet::new();
        let mut indices = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                if node >= 0 {
                    return Err("cyclic or reused BSP node".into());
                }
                continue;
            }
            if node >= 0 {
                let n = nodes
                    .get(node as usize * 32..node as usize * 32 + 32)
                    .ok_or("model node range")?;
                stack.push(i(n, 4));
                stack.push(i(n, 8));
            } else {
                let id = node
                    .checked_neg()
                    .and_then(|x| x.checked_sub(1))
                    .ok_or("invalid negative node")? as usize;
                let leaf = leaves
                    .get(id * stride..id * stride + stride)
                    .ok_or("model leaf range")?;
                let first = u(leaf, 24);
                let count = u(leaf, 26);
                let refs = refs
                    .get(first * 2..(first + count) * 2)
                    .ok_or("leafbrush range")?;
                for a in refs.chunks_exact(2) {
                    let n = u(a, 0);
                    if n >= brushes {
                        return Err("leaf brush index".into());
                    }
                    indices.insert(n);
                }
            }
        }
        out.push(indices.into_iter().collect());
    }
    Ok(out)
}
pub fn rotation(angles: glam::Vec3) -> glam::Mat3 {
    glam::Mat3::from_rotation_z(angles.y.to_radians())
        * glam::Mat3::from_rotation_y(angles.x.to_radians())
        * glam::Mat3::from_rotation_x(angles.z.to_radians())
}
