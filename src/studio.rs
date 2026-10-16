//! Reference-pose MDL 44/46/48 + IDSV VVD4 + packed DX90 VTX7, LOD0/default body.
//! VVD vertices are model-space bind geometry, including non-static models.
//! No animation, skinning updates, flexes, compressed vertices or material LOD replacement.
use glam::Vec3;
#[derive(Clone, Copy, Debug)]
pub struct StudioVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: [f32; 2],
    pub material: usize,
    pub weights: [f32; 3],
    pub bones: [u8; 3],
}
pub struct StudioModel {
    pub vertices: Vec<StudioVertex>,
    pub materials: Vec<Vec<String>>,
    pub skins: Vec<Vec<usize>>,
}
struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    fn range(&self, o: usize, n: usize) -> Result<&'a [u8], String> {
        self.0
            .get(o..o.checked_add(n).ok_or("studio range overflow")?)
            .ok_or("studio byte bounds".into())
    }
    fn i(&self, o: usize) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.range(o, 4)?.try_into().unwrap()))
    }
    fn n(&self, o: usize) -> Result<usize, String> {
        let n = self.i(o)?;
        if !(0..=2_000_000).contains(&n) {
            return Err("studio negative/oversized count or offset".into());
        }
        Ok(n as usize)
    }
    fn u(&self, o: usize) -> Result<usize, String> {
        Ok(u16::from_le_bytes(self.range(o, 2)?.try_into().unwrap()) as usize)
    }
    fn f(&self, o: usize) -> Result<f32, String> {
        let v = f32::from_le_bytes(self.range(o, 4)?.try_into().unwrap());
        if !v.is_finite() {
            return Err("nonfinite studio float".into());
        }
        Ok(v)
    }
    fn vec(&self, o: usize) -> Result<Vec3, String> {
        Ok(Vec3::new(self.f(o)?, self.f(o + 4)?, self.f(o + 8)?))
    }
    fn array(
        &self,
        base: usize,
        count: usize,
        offset: usize,
        stride: usize,
    ) -> Result<Vec<usize>, String> {
        let n = self.n(base + count)?;
        let p = base
            .checked_add(self.n(base + offset)?)
            .ok_or("studio offset overflow")?;
        self.range(p, n * stride)?;
        Ok((0..n).map(|j| p + j * stride).collect())
    }
    fn string(&self, o: usize) -> Result<String, String> {
        let b = self.0.get(o..).ok_or("studio string bounds")?;
        let end = b
            .iter()
            .position(|v| *v == 0)
            .ok_or("studio string terminator")?;
        Ok(std::str::from_utf8(&b[..end])
            .map_err(|e| e.to_string())?
            .replace('\\', "/")
            .to_ascii_lowercase())
    }
}
pub fn decode(mdl: &[u8], vvd: &[u8], vtx: &[u8]) -> Result<StudioModel, String> {
    let m = Bytes(mdl);
    let v = Bytes(vvd);
    let t = Bytes(vtx);
    m.range(0, 240)?;
    v.range(0, 64)?;
    t.range(0, 36)?;
    let id = m.range(0, 4)?;
    let version = m.i(4)?;
    let declared_len = m.i(76)?;
    if id != b"IDST"
        || ![44, 46, 48].contains(&version)
        || declared_len < 0
        || declared_len as usize != mdl.len()
    {
        return Err(format!("unsupported MDL header/version/length: id={} version={version} declared_len={declared_len} file_len={} (supported versions: 44,46,48)", String::from_utf8_lossy(id).escape_debug(), mdl.len()));
    }
    // VVD positions/normals are already in model-space reference (bind) pose.
    // Bone animation is deliberately not evaluated here.
    if v.range(0, 4)? != b"IDSV" || v.i(4)? != 4 || t.i(0)? != 7 {
        return Err("unsupported VVD/VTX version".into());
    }
    if m.i(8)? != v.i(8)? || m.i(8)? != t.i(16)? {
        return Err("MDL/VVD/VTX checksum mismatch".into());
    }
    let lods = v.n(12)?;
    if !(1..=8).contains(&lods) || t.n(20)? != lods {
        return Err("studio LOD count mismatch".into());
    }
    let raw_count = v.n(16)?;
    let raw_start = v.n(56)?;
    v.range(raw_start, raw_count * 48)?;
    let mut order = vec![];
    for p in v.array(0, 48, 52, 12)? {
        if v.n(p)? >= lods {
            return Err("VVD fixup LOD index".into());
        }
        let start = v.n(p + 4)?;
        let count = v.n(p + 8)?;
        if start + count > raw_count || order.len() + count > raw_count {
            return Err("VVD fixup vertex range".into());
        }
        order.extend(start..start + count);
    }
    if order.is_empty() {
        order.extend(0..raw_count);
    } else if order.len() != raw_count {
        return Err("VVD fixup total mismatch".into());
    }
    let mut seen = vec![false; raw_count];
    for &j in &order {
        if std::mem::replace(&mut seen[j], true) {
            return Err("duplicate VVD fixup vertex".into());
        }
    }
    let mut source = Vec::with_capacity(raw_count);
    for j in order {
        let p = raw_start + j * 48;
        let normal = v.vec(p + 28)?;
        if normal.length_squared() < 0.01 {
            return Err("invalid studio normal".into());
        }
        source.push(StudioVertex {
            position: v.vec(p + 16)?,
            normal: normal.normalize(),
            uv: [v.f(p + 40)?, v.f(p + 44)?],
            material: 0,
            weights: [v.f(p)?, v.f(p + 4)?, v.f(p + 8)?],
            bones: v.range(p + 12, 3)?.try_into().unwrap(),
        });
    }
    let dirs = m
        .array(0, 212, 216, 4)?
        .into_iter()
        .map(|p| m.string(m.n(p)?))
        .collect::<Result<Vec<_>, _>>()?;
    let mut materials = vec![];
    for p in m.array(0, 204, 208, 64)? {
        let name = m.string(p + m.n(p)?)?;
        let mut candidates = vec![];
        for dir in &dirs {
            candidates.push(crate::assets::normalise(&format!(
                "{}{name}",
                if dir.is_empty() {
                    String::new()
                } else {
                    format!("{}/", dir.trim_end_matches('/'))
                }
            ))?);
        }
        if candidates.is_empty() {
            candidates.push(crate::assets::normalise(&name)?);
        }
        materials.push(candidates);
    }
    let refs = m.n(220)?;
    let families = m.n(224)?;
    let skin_start = m.n(228)?;
    m.range(skin_start, refs * families * 2)?;
    if refs == 0 || families == 0 || materials.is_empty() {
        return Err("studio missing skin/material table".into());
    }
    let mut skins = vec![];
    for k in 0..families {
        let mut skin = vec![];
        for j in 0..refs {
            let id = m.u(skin_start + (k * refs + j) * 2)?;
            if id >= materials.len() {
                return Err("studio skin texture index".into());
            }
            skin.push(id);
        }
        skins.push(skin);
    }
    let bodies = m.array(0, 232, 236, 16)?;
    let tbodies = t.array(0, 28, 32, 8)?;
    if bodies.len() != tbodies.len() {
        return Err("MDL/VTX body count mismatch".into());
    }
    // Material replacements are relative packed lists, one per LOD. LOD0 only.
    let replacements = t.n(24)?;
    t.range(replacements, lods * 8)?;
    if t.n(replacements)? != 0 {
        return Err("VTX LOD0 material replacements unsupported".into());
    }
    let mut vertices = vec![];
    for (body, tb) in bodies.into_iter().zip(tbodies) {
        let models = m.array(body, 4, 12, 148)?;
        let tmodels = t.array(tb, 0, 4, 8)?;
        if models.len() != tmodels.len() {
            return Err("MDL/VTX model count mismatch".into());
        }
        let Some((&model, &tm)) = models.first().zip(tmodels.first()) else {
            continue;
        };
        let numvertices = m.n(model + 80)?;
        let vertexindex = m.n(model + 84)?;
        if vertexindex % 48 != 0 || vertexindex / 48 + numvertices > source.len() {
            return Err("MDL model vertex range".into());
        }
        let meshes = m.array(model, 72, 76, 116)?;
        let tlods = t.array(tm, 0, 4, 12)?;
        if tlods.len() != lods {
            return Err("VTX model LOD mismatch".into());
        }
        let tmeshes = t.array(tlods[0], 0, 4, 9)?;
        if meshes.len() != tmeshes.len() {
            return Err("MDL/VTX mesh count mismatch".into());
        }
        for (mesh, mt) in meshes.into_iter().zip(tmeshes) {
            let material = m.n(mesh)?;
            let count = m.n(mesh + 8)?;
            let offset = m.n(mesh + 12)?;
            if material >= refs || offset + count > numvertices {
                return Err("MDL mesh vertex/material index".into());
            }
            if m.n(mesh + 16)? != 0 {
                return Err("MDL flex mesh unsupported".into());
            }
            for sg in t.array(mt, 0, 4, 25)? {
                let tv = t.array(sg, 0, 4, 9)?;
                let indices = t.array(sg, 8, 12, 2)?;
                let strips = t.array(sg, 16, 20, 27)?;
                let mut group = vec![];
                for p in tv {
                    let id = t.u(p + 4)?;
                    if id >= count {
                        return Err("VTX original mesh vertex index".into());
                    }
                    let mut vert = source[vertexindex / 48 + offset + id];
                    vert.material = material;
                    group.push(vert);
                }
                let ids = indices
                    .into_iter()
                    .map(|p| t.u(p))
                    .collect::<Result<Vec<_>, _>>()?;
                if ids.iter().any(|&j| j >= group.len()) {
                    return Err("VTX stripgroup vertex index".into());
                }
                for strip in strips {
                    let n = t.n(strip)?;
                    let start = t.n(strip + 4)?;
                    let vn = t.n(strip + 8)?;
                    let vo = t.n(strip + 12)?;
                    if vo + vn > group.len() {
                        return Err("VTX strip vertex range".into());
                    }
                    let ids = ids.get(start..start + n).ok_or("VTX strip index range")?;
                    let flags = t.range(strip + 18, 1)?[0];
                    if flags == 1 {
                        if n % 3 != 0 {
                            return Err("VTX triangle list count".into());
                        }
                        for tri in ids.chunks_exact(3) {
                            vertices.extend(tri.iter().map(|&j| group[j]));
                        }
                    } else if flags == 2 {
                        for (k, tri) in ids.windows(3).enumerate() {
                            if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
                                continue;
                            }
                            let tri = if k % 2 == 0 {
                                [tri[0], tri[1], tri[2]]
                            } else {
                                [tri[1], tri[0], tri[2]]
                            };
                            vertices.extend(tri.into_iter().map(|j| group[j]));
                        }
                    } else {
                        return Err("unsupported VTX strip flags".into());
                    }
                    if vertices.len() > 6_000_000 {
                        return Err("studio triangle limit".into());
                    }
                }
            }
        }
    }
    if vertices.is_empty() {
        return Err("no static model geometry".into());
    }
    Ok(StudioModel {
        vertices,
        materials,
        skins,
    })
}
