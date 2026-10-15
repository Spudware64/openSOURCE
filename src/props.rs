//! Validated Portal sprp game-lump v5/v6. File offsets are BSP-absolute.
use glam::Vec3;
pub struct StaticProp {
    pub model: u16,
    pub origin: Vec3,
    pub angles: Vec3,
    pub skin: usize,
    pub solid: u8,
}
pub struct StaticProps {
    pub version: u16,
    pub models: Vec<String>,
    pub instances: Vec<StaticProp>,
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
}
fn vector(b: &[u8], o: usize) -> Vec3 {
    Vec3::from_array(std::array::from_fn(|n| {
        f32::from_le_bytes(b[o + n * 4..o + n * 4 + 4].try_into().unwrap())
    }))
}
fn array<'a>(b: &'a [u8], p: &mut usize, stride: usize) -> Result<&'a [u8], String> {
    let a = b.get(*p..*p + 4).ok_or("sprp count bounds")?;
    let count = i(a, 0);
    if !(0..=1_000_000).contains(&count) {
        return Err("sprp count limit".into());
    }
    *p += 4;
    let end = p
        .checked_add(count as usize * stride)
        .ok_or("sprp overflow")?;
    let a = b.get(*p..end).ok_or("sprp array bounds")?;
    *p = end;
    Ok(a)
}
pub fn static_props(b: &[u8]) -> Result<StaticProps, String> {
    let g = crate::lump(b, 35, 1)?;
    let empty = || StaticProps {
        version: 0,
        models: vec![],
        instances: vec![],
    };
    if g.is_empty() {
        return Ok(empty());
    }
    if i(b, 8 + 35 * 16 + 8) != 0 {
        return Err("unsupported game lump directory version".into());
    }
    let mut p = 0;
    let entries = array(g, &mut p, 16)?;
    let mut result = None;
    for a in entries.chunks_exact(16) {
        if &a[..4] != b"prps" {
            continue;
        }
        if result.is_some() {
            return Err("duplicate sprp game lump".into());
        }
        if u(a, 4) != 0 {
            return Err("compressed sprp unsupported".into());
        }
        let version = u(a, 6);
        let stride = match version {
            5 => 60,
            6 => 64,
            _ => return Err(format!("unsupported sprp version {version}")),
        };
        let offset = i(a, 8);
        let length = i(a, 12);
        if offset < 0 || length < 0 {
            return Err("negative sprp bounds".into());
        }
        let data = b
            .get(offset as usize..offset as usize + length as usize)
            .ok_or("sprp file bounds")?;
        let mut p = 0;
        let mut models = vec![];
        for name in array(data, &mut p, 128)?.chunks_exact(128) {
            let end = name
                .iter()
                .position(|v| *v == 0)
                .ok_or("sprp model terminator")?;
            let path = crate::assets::normalise(
                std::str::from_utf8(&name[..end]).map_err(|e| e.to_string())?,
            )?;
            if !path.starts_with("models/") || !path.ends_with(".mdl") {
                return Err("sprp model path".into());
            }
            models.push(path);
        }
        let leaf_refs = array(data, &mut p, 2)?;
        let leaf_count = crate::lump(b, 10, 1)?.len()
            / match i(b, 8 + 10 * 16 + 8) {
                0 => 56,
                1 => 32,
                _ => return Err("sprp leaf layout".into()),
            };
        for a in leaf_refs.chunks_exact(2) {
            if u(a, 0) as usize >= leaf_count {
                return Err("sprp leaf index".into());
            }
        }
        let mut instances = vec![];
        for a in array(data, &mut p, stride)?.chunks_exact(stride) {
            let model = u(a, 24);
            let first = u(a, 26) as usize;
            let count = u(a, 28) as usize;
            let skin = i(a, 32);
            if model as usize >= models.len() || first + count > leaf_refs.len() / 2 || skin < 0 {
                return Err("sprp model/leaf/skin index".into());
            }
            let origin = vector(a, 0);
            let angles = vector(a, 12);
            if !origin.is_finite()
                || !angles.is_finite()
                || (a[31] & 2 != 0 && !vector(a, 44).is_finite())
            {
                return Err("nonfinite sprp transform".into());
            }
            instances.push(StaticProp {
                model,
                origin,
                angles,
                skin: skin as usize,
                solid: a[30],
            });
        }
        if p != data.len() {
            return Err("sprp trailing bytes/layout mismatch".into());
        }
        result = Some(StaticProps {
            version,
            models,
            instances,
        });
    }
    Ok(result.unwrap_or_else(empty))
}
