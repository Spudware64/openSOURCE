//! Bounded Source studio local single-blend animation. No procedural bones, IK,
//! flex, delta layers, external ANI blocks or sectioned/frame-compressed streams.
use crate::studio::StudioVertex;
use glam::{Mat4, Quat, Vec3, Vec4};
#[derive(Clone)]
struct Bone {
    parent: Option<usize>,
    pos: Vec3,
    rot: Vec3,
    quat: Quat,
    ps: Vec3,
    rs: Vec3,
    inverse: Mat4,
}
#[derive(Clone)]
pub struct Clip {
    pub name: String,
    pub fps: f32,
    pub looping: bool,
    frames: Vec<Vec<(Vec3, Quat)>>,
}
impl Clip {
    pub fn duration(&self) -> f32 {
        (self.frames.len() - 1) as f32 / self.fps
    }
}
#[derive(Clone)]
pub struct Skeleton {
    bones: Vec<Bone>,
    pub clips: Vec<Clip>,
    pub unsupported: Vec<String>,
    pub initial_sequence: Option<String>,
}
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn bytes(&self, p: usize, n: usize) -> Result<&[u8], String> {
        self.0
            .get(p..p.checked_add(n).ok_or("animation overflow")?)
            .ok_or("animation bounds".into())
    }
    fn i(&self, p: usize) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.bytes(p, 4)?.try_into().unwrap()))
    }
    fn n(&self, p: usize) -> Result<usize, String> {
        let n = self.i(p)?;
        if !(0..=2_000_000).contains(&n) {
            return Err("animation count/offset".into());
        }
        Ok(n as usize)
    }
    fn s(&self, p: usize) -> Result<i16, String> {
        Ok(i16::from_le_bytes(self.bytes(p, 2)?.try_into().unwrap()))
    }
    fn f(&self, p: usize) -> Result<f32, String> {
        let x = f32::from_le_bytes(self.bytes(p, 4)?.try_into().unwrap());
        if !x.is_finite() {
            return Err("nonfinite animation".into());
        }
        Ok(x)
    }
    fn v(&self, p: usize) -> Result<Vec3, String> {
        Ok(Vec3::new(self.f(p)?, self.f(p + 4)?, self.f(p + 8)?))
    }
    fn name(&self, p: usize) -> Result<String, String> {
        let a = self.0.get(p..).ok_or("animation name bounds")?;
        let end = a
            .iter()
            .position(|b| *b == 0)
            .ok_or("animation name terminator")?;
        Ok(std::str::from_utf8(&a[..end])
            .map_err(|e| e.to_string())?
            .to_ascii_lowercase())
    }
    fn q(&self, p: usize) -> Result<Quat, String> {
        let q = Quat::from_xyzw(self.f(p)?, self.f(p + 4)?, self.f(p + 8)?, self.f(p + 12)?);
        if (q.length() - 1.).abs() > 0.05 {
            return Err("invalid bone quaternion".into());
        }
        Ok(q.normalize())
    }
    fn packed_q(&self, p: usize, wide: bool) -> Result<Quat, String> {
        let (x, y, z, neg) = if wide {
            let b = u64::from_le_bytes(self.bytes(p, 8)?.try_into().unwrap());
            let cv = |v: u64| ((v & 0x1fffff) as f32 - 1048576.) / 1048576.5;
            (cv(b), cv(b >> 21), cv(b >> 42), b >> 63 != 0)
        } else {
            let a = self.s(p)? as u16;
            let b = self.s(p + 2)? as u16;
            let c = self.s(p + 4)? as u16;
            (
                (a as f32 - 32768.) / 32768.,
                (b as f32 - 32768.) / 32768.,
                ((c & 32767) as f32 - 16384.) / 16384.,
                c & 32768 != 0,
            )
        };
        let w2 = 1. - x * x - y * y - z * z;
        if w2 < -0.001 {
            return Err("invalid packed quaternion".into());
        }
        Ok(Quat::from_xyzw(x, y, z, w2.max(0.).sqrt() * if neg { -1. } else { 1. }).normalize())
    }
    fn half(&self, p: usize) -> Result<f32, String> {
        let h = self.s(p)? as u16;
        let sign = if h & 32768 != 0 { -1. } else { 1. };
        let e = (h >> 10) & 31;
        let m = h & 1023;
        let v = if e == 0 {
            sign * (m as f32) * 2f32.powi(-24)
        } else if e == 31 {
            return Err("nonfinite animation half".into());
        } else {
            sign * (1. + m as f32 / 1024.) * 2f32.powi(e as i32 - 15)
        };
        Ok(v)
    }
    fn rle(&self, base: usize, axis: usize, mut frame: usize) -> Result<f32, String> {
        let off = self.s(base + axis * 2)?;
        if off == 0 {
            return Ok(0.);
        }
        if off < 0 {
            return Err("negative animation channel offset".into());
        }
        let mut p = base + off as usize;
        for _ in 0..65536 {
            let a = self.bytes(p, 2)?;
            let valid = a[0] as usize;
            let total = a[1] as usize;
            if valid == 0 || total == 0 || valid > total {
                return Err("invalid animation RLE run".into());
            }
            self.bytes(p + 2, valid * 2)?;
            if frame < total {
                return Ok(self.s(p + 2 * (frame + 1).min(valid))? as f32);
            }
            frame -= total;
            p += 2 * (valid + 1);
        }
        Err("animation RLE limit".into())
    }
}
fn euler(r: Vec3) -> Quat {
    Quat::from_rotation_z(r.z) * Quat::from_rotation_y(r.y) * Quat::from_rotation_x(r.x)
}
impl Skeleton {
    pub fn decode(mdl: &[u8], _ani: Option<&[u8]>) -> Result<Self, String> {
        let r = Reader(mdl);
        if r.bytes(0, 4)? != b"IDST" || ![44, 46, 48].contains(&r.i(4)?) || r.n(76)? != mdl.len() {
            return Err("unsupported animated MDL version/length".into());
        }
        let count = r.n(156)?;
        if count == 0 || count > 256 {
            return Err("animation bone count".into());
        }
        let start = r.n(160)?;
        r.bytes(start, count * 216)?;
        let mut bones = vec![];
        for id in 0..count {
            let p = start + id * 216;
            let parent = r.i(p + 4)?;
            if parent < -1 || parent >= id as i32 {
                return Err("invalid/cyclic bone hierarchy".into());
            }
            if r.i(p + 164)? != 0 {
                return Err("procedural bone animation unsupported".into());
            }
            let mut rows = [[0.; 4]; 3];
            for (y, row) in rows.iter_mut().enumerate() {
                for (x, v) in row.iter_mut().enumerate() {
                    *v = r.f(p + 96 + (y * 4 + x) * 4)?;
                }
            }
            let inverse = Mat4::from_cols(
                Vec4::new(rows[0][0], rows[1][0], rows[2][0], 0.),
                Vec4::new(rows[0][1], rows[1][1], rows[2][1], 0.),
                Vec4::new(rows[0][2], rows[1][2], rows[2][2], 0.),
                Vec4::new(rows[0][3], rows[1][3], rows[2][3], 1.),
            );
            bones.push(Bone {
                parent: (parent >= 0).then_some(parent as usize),
                pos: r.v(p + 32)?,
                quat: r.q(p + 44)?,
                rot: r.v(p + 60)?,
                ps: r.v(p + 72)?,
                rs: r.v(p + 84)?,
                inverse,
            });
        }
        let na = r.n(180)?;
        let ap = r.n(184)?;
        let ns = r.n(188)?;
        let sp = r.n(192)?;
        if na > 4096 || ns > 4096 {
            return Err("animation sequence limit".into());
        }
        r.bytes(ap, na * 100)?;
        r.bytes(sp, ns * 212)?;
        let mut out = Self {
            bones,
            clips: vec![],
            unsupported: vec![],
            initial_sequence: None,
        };
        for j in 0..ns {
            let p = sp + j * 212;
            let name = r.name(p + r.n(p + 4)?)?;
            if j == 0 {
                out.initial_sequence = Some(name.clone());
            }
            let result: Result<Clip, String> = (|| {
                if r.i(p + 56)? != 1
                    || r.i(p + 68)? != 1
                    || r.i(p + 72)? != 1
                    || r.i(p + 148)? != 0
                    || r.i(p + 12)? & 16 != 0
                {
                    return Err("blended/delta/autolayer sequence unsupported".into());
                }
                let aid = r.s(p + r.n(p + 60)?)?;
                if aid < 0 || aid as usize >= na {
                    return Err("sequence animation index".into());
                }
                let a = ap + aid as usize * 100;
                let frames = r.n(a + 16)?;
                let fps = r.f(a + 8)?;
                if frames == 0 || frames > 4096 || fps <= 0. || frames * out.bones.len() > 262144 {
                    return Err("animation frame/fps limit".into());
                }
                if r.i(a + 52)? != 0 {
                    return Err("external ANI blocks unsupported".into());
                }
                if r.i(a + 80)? != 0
                    || r.i(a + 84)? != 0
                    || r.i(a + 12)? & (16 | 64) != 0
                    || r.i(a + 72)? != 0
                {
                    return Err(
                        "sectioned/frame-compressed/delta/local-hierarchy animation unsupported"
                            .into(),
                    );
                }
                let mut poses = vec![];
                for frame in 0..frames {
                    let mut pose = out
                        .bones
                        .iter()
                        .map(|b| (b.pos, b.quat))
                        .collect::<Vec<_>>();
                    let index = r.n(a + 56)?;
                    if index != 0 {
                        let mut t = a + index;
                        let mut seen = vec![false; count];
                        for _ in 0..count {
                            let h = r.bytes(t, 4)?;
                            let id = h[0] as usize;
                            let flags = h[1];
                            if id >= count || seen[id] {
                                return Err("animation bone index/duplicate".into());
                            }
                            seen[id] = true;
                            if flags & !47 != 0 || flags & 16 != 0 {
                                return Err("unsupported bone animation flags".into());
                            }
                            let bone = &out.bones[id];
                            let mut pos = bone.pos;
                            let mut quat = bone.quat;
                            let data = t + 4;
                            if flags & 2 != 0 {
                                quat = r.packed_q(data, false)?;
                            } else if flags & 32 != 0 {
                                quat = r.packed_q(data, true)?;
                            } else if flags & 8 != 0 {
                                let rot = bone.rot
                                    + Vec3::new(
                                        r.rle(data, 0, frame)? * bone.rs.x,
                                        r.rle(data, 1, frame)? * bone.rs.y,
                                        r.rle(data, 2, frame)? * bone.rs.z,
                                    );
                                quat = euler(rot);
                            }
                            if flags & 1 != 0 {
                                let q = data
                                    + if flags & 2 != 0 {
                                        6
                                    } else if flags & 32 != 0 {
                                        8
                                    } else {
                                        0
                                    };
                                pos = Vec3::new(r.half(q)?, r.half(q + 2)?, r.half(q + 4)?);
                            } else if flags & 4 != 0 {
                                let q = data + if flags & 8 != 0 { 6 } else { 0 };
                                pos += Vec3::new(
                                    r.rle(q, 0, frame)? * bone.ps.x,
                                    r.rle(q, 1, frame)? * bone.ps.y,
                                    r.rle(q, 2, frame)? * bone.ps.z,
                                );
                            }
                            pose[id] = (pos, quat);
                            let next = r.s(t + 2)?;
                            if next == 0 {
                                break;
                            }
                            if next < 4 {
                                return Err("invalid animation next offset".into());
                            }
                            t += next as usize;
                            if seen.iter().all(|s| *s) {
                                return Err("animation chain exceeds bones".into());
                            }
                        }
                    }
                    poses.push(pose);
                }
                Ok(Clip {
                    name: name.clone(),
                    fps,
                    looping: r.i(p + 12)? & 1 != 0,
                    frames: poses,
                })
            })();
            match result {
                Ok(c) => out.clips.push(c),
                Err(e) => out.unsupported.push(format!("{name}: {e}")),
            }
        }
        Ok(out)
    }
    pub fn clip(&self, name: &str) -> Result<&Clip, String> {
        self.clips
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("unknown/unsupported sequence {name}"))
    }
    pub fn skin(
        &self,
        vertices: &[StudioVertex],
        sequence: &str,
        time: f32,
    ) -> Result<Vec<StudioVertex>, String> {
        if !time.is_finite() || time < 0. {
            return Err("invalid animation time".into());
        }
        let c = self.clip(sequence)?;
        let end = (c.frames.len() - 1) as f32;
        let f = if c.looping && end > 0. {
            (time * c.fps) % end
        } else {
            (time * c.fps).min(end)
        };
        let a = f.floor() as usize;
        let b = (a + 1).min(c.frames.len() - 1);
        let frac = f - a as f32;
        let mut world = vec![Mat4::IDENTITY; self.bones.len()];
        let mut skin = world.clone();
        for (id, bone) in self.bones.iter().enumerate() {
            let (p, q) = c.frames[a][id];
            let (p2, q2) = c.frames[b][id];
            let m = Mat4::from_rotation_translation(q.slerp(q2, frac), p.lerp(p2, frac));
            world[id] = bone.parent.map(|p| world[p] * m).unwrap_or(m);
            skin[id] = world[id] * bone.inverse;
        }
        vertices
            .iter()
            .map(|v| {
                let mut p = Vec3::ZERO;
                let mut n = Vec3::ZERO;
                let mut sum = 0.;
                for k in 0..3 {
                    let w = v.weights[k];
                    if !w.is_finite() || w < 0. {
                        return Err("invalid skin weight".into());
                    }
                    if w == 0. {
                        continue;
                    }
                    let m = skin.get(v.bones[k] as usize).ok_or("skin bone index")?;
                    p += m.transform_point3(v.position) * w;
                    n += m.transform_vector3(v.normal) * w;
                    sum += w;
                }
                if (sum - 1.).abs() > 0.02 || n.length_squared() < 0.00001 {
                    return Err("invalid skin weight sum/normal".into());
                }
                let mut out = *v;
                out.position = p;
                out.normal = n.normalize();
                Ok(out)
            })
            .collect()
    }
}
