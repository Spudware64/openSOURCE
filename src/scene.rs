//! Source 1 face UVs, VMT/VTF diffuse materials and RGBExp32 baked irradiance.
use crate::{
    assets::Assets,
    kv::{self, Value},
    texture::{decode_vtf, Texture},
};
use glam::Vec3;
use rayon::prelude::*;
pub const LIGHT_SIZE: usize = 4096;
pub const TILE: usize = 256;
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub light_uv: [f32; 2],
    pub layer: u32,
    pub model: u32,
    pub unlit: f32,
    pub normal: [f32; 3],
}
pub struct AnimationInstance {
    pub name: String,
    pub range: std::ops::Range<usize>,
    pub skeleton: crate::animation::Skeleton,
    pub initial: String,
    pub solid: bool,
    pub updated: bool,
    last_pose: (String, f32),
    vertices: crate::animation::SkinMesh,
    rotation: glam::Mat3,
    offset: Vec3,
}
pub struct Scene {
    pub pellet_template: Vec<MeshVertex>,
    pub scene_durations: std::collections::HashMap<u32, f32>,
    pub animations: Vec<AnimationInstance>,
    pub static_props: usize,
    pub dynamic_props: usize,
    pub skipped_dynamic_props: usize,
    pub skipped_props: usize,
    pub prop_triangles: usize,
    pub displacement_triangles: usize,
    pub displacement_faces: usize,
    pub vertices: Vec<MeshVertex>,
    pub textures: Vec<Vec<u8>>,
    pub lightmap: Vec<u8>,
    pub filtered_faces: usize,
    pub lightmapped_faces: usize,
    pub warnings: Vec<String>,
    pub origins: Vec<Vec3>,
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> usize {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap()) as usize
}
struct Material {
    texture: Texture,
    unlit: bool,
    alpha_test: bool,
    translucent: bool,
}
fn material(fs: &Assets, name: &str, depth: usize) -> Result<Material, String> {
    if depth > 8 {
        return Err("VMT include depth limit".into());
    }
    let path = if name.ends_with(".vmt") {
        name.to_owned()
    } else {
        format!("materials/{name}.vmt")
    };
    let bytes = fs.read(&path)?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let tree = kv::parse(text)?;
    let (shader, value) = tree.first().ok_or("empty VMT")?;
    let Value::Block(fields) = value else {
        return Err("VMT shader block".into());
    };
    if shader.eq_ignore_ascii_case("patch") {
        let include = kv::text(fields, "include").ok_or("VMT patch missing include")?;
        let mut m = material(fs, include, depth + 1)?;
        for kind in ["insert", "replace"] {
            if let Some(fields) = kv::child(fields, kind) {
                if let Some(value) = kv::text(fields, "$alphatest") {
                    m.alpha_test = value == "1";
                }
                if let Some(value) = kv::text(fields, "$translucent") {
                    m.translucent = value == "1";
                }
                if let Some(base) = kv::text(fields, "$basetexture") {
                    m.texture = decode_vtf(&fs.read(&format!("materials/{base}.vtf"))?)?;
                }
            }
        }
        return Ok(m);
    }
    if shader.eq_ignore_ascii_case("Refract") {
        // A bounded clear-glass approximation: never interpret a normal map or
        // envmap mask as opaque diffuse. Screen-space distortion is not implemented.
        return Ok(Material {
            texture: Texture {
                width: 1,
                height: 1,
                rgba: vec![200, 220, 230, 32],
            },
            unlit: true,
            alpha_test: false,
            translucent: true,
        });
    }
    let base = kv::text(fields, "$basetexture")
        .ok_or_else(|| format!("{path}: unsupported shader/no basetexture {shader}"))?;
    let base = base.trim_end_matches(".vtf").replace('\\', "/");
    Ok(Material {
        texture: decode_vtf(&fs.read(&format!("materials/{base}.vtf"))?)?,
        unlit: shader.eq_ignore_ascii_case("UnlitGeneric"),
        alpha_test: kv::text(fields, "$alphatest") == Some("1"),
        translucent: kv::text(fields, "$translucent") == Some("1"),
    })
}
fn resample(m: &Material) -> Vec<u8> {
    let t = &m.texture;
    let mut out = vec![0; TILE * TILE * 4];
    for y in 0..TILE {
        for x in 0..TILE {
            let n = (y * t.height / TILE * t.width + x * t.width / TILE) * 4;
            out[(y * TILE + x) * 4..(y * TILE + x) * 4 + 4].copy_from_slice(&t.rgba[n..n + 4]);
        }
    }
    // Source opaque materials frequently use diffuse alpha as an env/specular mask.
    // It is not coverage unless the VMT explicitly requests alpha test/translucency.
    if !m.alpha_test && !m.translucent {
        for p in out.chunks_exact_mut(4) {
            p[3] = 255;
        }
    }
    out
}
impl Scene {
    /// Original LOD0 geometry, including the authored glow meshes. Assets stay cached.
    pub fn energy_ball(fs: &Assets) -> Result<(Vec<MeshVertex>, Vec<Vec<u8>>), String> {
        let stem = "models/effects/combineball";
        let model = crate::studio::decode(&fs.read(&format!("{stem}.mdl"))?, &fs.read(&format!("{stem}.vvd"))?, &fs.read(&format!("{stem}.dx90.vtx"))?)?;
        let skin = model.skins.first().ok_or("combineball skin")?;
        let mut textures = vec![];
        for candidates in &model.materials {
            let m = candidates.iter().find_map(|name| material(fs, name, 0).ok()).ok_or_else(|| format!("combineball material unavailable: {candidates:?}"))?;
            textures.push(resample(&m));
        }
        let vertices = model.vertices.iter().map(|v| {
            let names = &model.materials[skin[v.material]];
            let has = |s:&str| names.iter().any(|n| n.ends_with(s));
            let scroll = if has("comball_sphere") { [(-80f32).to_radians().cos(), (-80f32).to_radians().sin()] } else if has("comball_tape") {[-0.70710677,-0.70710677]} else {[0.;2]};
            MeshVertex { position:v.position.to_array(), normal:v.normal.to_array(), uv:v.uv, light_uv:scroll, layer:skin[v.material] as u32, model:0, unlit:if has("comball_glow1") {-3.} else if has("comball_glow2") {-4.} else {-2.} }
        }).collect();
        Ok((vertices,textures))
    }
    /// Original weapon-script viewmodel, posed once in authored idle coordinates.
    /// Kept separate from BSP geometry and its animation/upload hot path.
    pub fn weapon_viewmodel(fs: &Assets, weapon: &str) -> Result<Self, String> {
        if !weapon.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err("invalid weapon script name".into());
        }
        let script = fs.read(&format!("scripts/{weapon}.txt"))?;
        let tree = kv::parse(std::str::from_utf8(&script).map_err(|e| e.to_string())?)?;
        let fields = kv::child(&tree, "WeaponData").ok_or("weapon script block")?;
        let path = kv::text(fields, "viewmodel").ok_or("weapon viewmodel")?;
        let stem = path.trim_end_matches(".mdl");
        let mdl = fs.read(path)?;
        let model = crate::studio::decode(&mdl, &fs.read(&format!("{stem}.vvd"))?, &fs.read(&format!("{stem}.dx90.vtx"))?)?;
        let skeleton = crate::animation::Skeleton::decode(&mdl, None)?;
        let idle = skeleton.clips.iter().find(|c| c.name.eq_ignore_ascii_case("idle")).or_else(|| skeleton.clips.iter().find(|c| c.name.to_lowercase().contains("idle"))).ok_or_else(|| format!("viewmodel idle unavailable: {:?}", skeleton.unsupported))?;
        let posed = skeleton.skin(&model.vertices, &idle.name, 0.)?;
        let mut textures = vec![];
        // Resolve all authored CD-material candidates: the original hands live
        // under v_models/v_portalgun, not the first legacy search directory.
        for (index, candidates) in model.materials.iter().enumerate() {
            if !posed.iter().any(|v| model.skins[0][v.material] == index) { textures.push(vec![0; TILE*TILE*4]); continue; }
            let m = candidates.iter().find_map(|name| material(fs,name,0).ok()).or_else(|| {
                // Shipped v_hands.vmt references a nonexistent HL2-era directory;
                // its original diffuse is actually alongside this Portal VMT.
                candidates.iter().filter(|n|n.ends_with("/v_hands")).find_map(|n| {
                    let texture=decode_vtf(&fs.read(&format!("materials/{n}.vtf")).ok()?).ok()?;
                    Some(Material {texture,unlit:false,alpha_test:false,translucent:false})
                })
            }).ok_or_else(||format!("missing viewmodel material {candidates:?}"))?;
            textures.push(resample(&m));
        }
        let skin = model.skins.first().ok_or("viewmodel skin")?;
        let vertices = posed.iter().map(|v| MeshVertex { position:v.position.to_array(), uv:v.uv, light_uv:[0.,0.], layer:skin[v.material] as u32, model:0, unlit:2., normal:v.normal.to_array() }).collect();
        let animation = AnimationInstance { name:"viewmodel".into(), range:0..posed.len(), initial:"idle".into(), solid:false, updated:true, last_pose:("idle".into(),0.), vertices:crate::animation::SkinMesh::new(&model.vertices), rotation:glam::Mat3::IDENTITY, offset:Vec3::ZERO, skeleton };
        Ok(Self { pellet_template:vec![], scene_durations:Default::default(), animations:vec![animation], static_props:0, dynamic_props:0, skipped_dynamic_props:0, skipped_props:0, prop_triangles:0, displacement_triangles:0, displacement_faces:0, vertices, textures, lightmap:vec![255; LIGHT_SIZE*LIGHT_SIZE*4], filtered_faces:0, lightmapped_faces:0, warnings:vec![], origins:vec![Vec3::ZERO] })
    }

    /// One-shot authored sequences return to idle; no synthetic sway or audio events.
    pub fn weapon_pose(&self, sequence: &str, elapsed: f32) -> Result<(String,f32),String> {
        let clip = self.animations.first().ok_or("missing weapon skeleton")?.skeleton.clip(sequence)?;
        if !clip.looping && elapsed > clip.duration() { Ok(("idle".into(),0.)) }
        else { Ok((sequence.into(),elapsed)) }
    }
    pub fn pose_weapon(&mut self, sequence: &str, elapsed: f32) -> Result<(),String> {
        let (sequence,time) = self.weapon_pose(sequence,elapsed)?;
        let a = self.animations.first_mut().ok_or("missing weapon skeleton")?;
        a.updated = a.last_pose != (sequence.clone(),time);
        if !a.updated {return Ok(());}
        let posed = a.skeleton.skin(&a.vertices.vertices,&sequence,time)?;
        for (dst,&index) in self.vertices.iter_mut().zip(&a.vertices.indices) {
            dst.position=posed[index].position.to_array();
            dst.normal=posed[index].normal.to_array();
        }
        a.last_pose=(sequence,time);
        Ok(())
    }

    pub fn animate(&mut self, runtime: &crate::entity::Runtime) -> Result<usize, String> {
        let mut updated = 0;
        for instance in &mut self.animations {
            instance.updated = false;
            let Some((sequence, time)) = runtime.animation_pose(&instance.name) else {
                continue;
            };
            let clip = instance.skeleton.clip(sequence)?;
            let duration = clip.duration();
            let time = if clip.looping && duration > 0. {
                time % duration
            } else {
                time.min(duration)
            };
            let model = self.vertices[instance.range.start].model as usize;
            let transform = runtime.entity_transform(&instance.name);
            let rotation = transform
                .map(glam::Mat3::from_mat4)
                .unwrap_or(instance.rotation);
            let offset = transform
                .map(|t| t.w_axis.truncate() - runtime.origins[model])
                .unwrap_or(instance.offset);
            let spatial_changed = !rotation.abs_diff_eq(instance.rotation, 0.00001)
                || !offset.abs_diff_eq(instance.offset, 0.0001);
            if instance.last_pose.0 == sequence && instance.last_pose.1 == time && !spatial_changed
            {
                continue;
            }
            instance.rotation = rotation;
            instance.offset = offset;
            let posed = instance.skeleton.skin(&instance.vertices.vertices, sequence, time)?;
            instance.last_pose = (sequence.to_owned(), time);
            instance.updated = true;
            updated += 1;
            for (dst, src) in self.vertices[instance.range.clone()].iter_mut().zip(instance.vertices.indices.iter().map(|&i| posed[i])) {
                dst.position = (instance.rotation * src.position + instance.offset).to_array();
                dst.normal = (instance.rotation * src.normal).to_array();
            }
        }
        Ok(updated)
    }
    pub fn load(b: &[u8], fs: &Assets) -> Result<Self, String> {
        Self::load_with_workers(b, fs, 4)
    }
    pub fn load_with_workers(b: &[u8], fs: &Assets, workers: usize) -> Result<Self, String> {
        crate::load(b)?;
        for (id, version) in [(2, 0), (6, 0), (8, 1), (43, 0), (44, 0)] {
            if i(b, 8 + id * 16 + 8) != version {
                return Err(format!("unsupported material/lightmap layout {id}"));
            }
        }
        if !(1..=16).contains(&workers) {
            return Err("asset workers must be 1..16".into());
        }
        let data = crate::lump(b, 2, 32)?;
        let info = crate::lump(b, 6, 72)?;
        let names = crate::lump(b, 43, 1)?;
        let table = crate::lump(b, 44, 4)?;
        let mut materials = vec![];
        for a in data.chunks_exact(32) {
            let index = i(a, 12);
            if index < 0 {
                return Err("negative material name index".into());
            }
            let a = table
                .get(index as usize * 4..index as usize * 4 + 4)
                .ok_or("material name table range")?;
            let offset = i(a, 0);
            if offset < 0 {
                return Err("negative material name offset".into());
            }
            let bytes = names.get(offset as usize..).ok_or("material name range")?;
            let n = bytes
                .iter()
                .position(|x| *x == 0)
                .ok_or("material name terminator")?;
            materials.push(
                std::str::from_utf8(&bytes[..n])
                    .map_err(|e| e.to_string())?
                    .replace('\\', "/")
                    .to_ascii_lowercase(),
            );
        }
        if materials.is_empty() || materials.len() > 1024 {
            return Err("material layer limit (1024)".into());
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|n| format!("source-assets-{n}"))
            .build()
            .map_err(|e| e.to_string())?;
        let decoded = pool.install(|| {
            materials
                .par_iter()
                .map(|name| {
                    if name.starts_with("tools/") {
                        Ok((vec![255; TILE * TILE * 4], false, false))
                    } else {
                        material(fs, name, 0).map(|m| (resample(&m), m.unlit, m.translucent))
                    }
                })
                .collect::<Vec<_>>()
        });
        let mut textures = vec![];
        let mut unlit = vec![];
        let mut translucent = vec![];
        let mut warnings = vec![];
        for (n, result) in decoded.into_iter().enumerate() {
            match result {
                Ok((t, u, blend)) => {
                    textures.push(t);
                    unlit.push(u);
                    translucent.push(blend);
                }
                Err(e) => {
                    warnings.push(format!("{}: {e}", materials[n]));
                    let mut t = vec![255; TILE * TILE * 4];
                    for y in 0..TILE {
                        for x in 0..TILE {
                            let p = (y * TILE + x) * 4;
                            t[p + 1] = if (x / 16 + y / 16) % 2 == 0 { 0 } else { 80 };
                        }
                    }
                    textures.push(t);
                    unlit.push(false);
                    translucent.push(false);
                }
            }
        }
        let vertices = crate::lump(b, 3, 12)?
            .chunks_exact(12)
            .map(|a| [f(a, 0), f(a, 4), f(a, 8)])
            .collect::<Vec<_>>();
        let edges = crate::lump(b, 12, 4)?
            .chunks_exact(4)
            .map(|a| [u(a, 0) as u16, u(a, 2) as u16])
            .collect::<Vec<_>>();
        let surfedges = crate::lump(b, 13, 4)?
            .chunks_exact(4)
            .map(|a| i(a, 0))
            .collect::<Vec<_>>();
        let faces = crate::lump(b, 7, 56)?;
        let models = crate::lump(b, 14, 48)?;
        if models.len() / 48 > 256 {
            return Err("brush model limit (256)".into());
        }
        let mut face_models = vec![0; faces.len() / 56];
        for (id, a) in models.chunks_exact(48).enumerate() {
            for n in i(a, 40)..i(a, 40) + i(a, 44) {
                face_models[n as usize] = id;
            }
        }
        let entities = crate::entity::parse_entities(crate::lump(b, 0, 1)?)?;
        let entity_runtime = crate::entity::Runtime::new(b)?;
        let origins = entity_runtime.origins.clone();
        let mut rotations = vec![glam::Mat3::IDENTITY; origins.len()];
        let mut hidden = vec![false; origins.len()];
        for e in &entities {
            if let Some(model) = e.model()? {
                if model >= origins.len() {
                    return Err("entity model range".into());
                }
                let angles = crate::entity::vector(e.get("angles").unwrap_or("0 0 0"))?;
                rotations[model] = crate::models::rotation(angles);
                let class = e.get("classname").unwrap_or("");
                hidden[model] = class.starts_with("trigger_")
                    || [
                        "func_areaportal",
                        "func_noportal_volume",
                        "func_portal_detector",
                    ]
                    .contains(&class)
                    || e.get("renderamt") == Some("0");
            }
        }
        for id in [26, 33, 48] {
            if i(b, 8 + id * 16 + 8) != 0 {
                return Err(format!("unsupported displacement lump version {id}"));
            }
        }
        let disp_info = crate::lump(b, 26, 176)?;
        let disp_verts = crate::lump(b, 33, 20)?;
        let disp_tris = crate::lump(b, 48, 2)?;
        let mut displacement_triangles = 0;
        let mut displacement_faces = 0;
        let lighting = crate::lump(b, 8, 4)?;
        let mut lightmap = vec![255; LIGHT_SIZE * LIGHT_SIZE * 4];
        let mut x = 2;
        let mut y = 2;
        let mut row = 0;
        let mut out = vec![];
        let mut filtered = 0;
        let mut lit = 0;
        for (fi, a) in faces.chunks_exact(56).enumerate() {
            let ti = u(a, 10);
            let tex = info
                .get(ti * 72..ti * 72 + 72)
                .ok_or("face texinfo range")?;
            let td = i(tex, 68);
            if td < 0 || td as usize >= materials.len() {
                return Err("texinfo texdata range".into());
            }
            let layer = td as usize;
            if i(tex, 64) & (0x80 | 0x4 | 0x2) != 0
                || materials[layer].starts_with("tools/")
                || hidden[face_models[fi]]
            {
                filtered += 1;
                continue;
            }
            if tex[..64].chunks_exact(4).any(|a| !f(a, 0).is_finite()) {
                return Err("nonfinite texinfo".into());
            }
            let first = i(a, 4) as usize;
            let count = u(a, 8);
            let polygon = crate::polygon(&vertices, &edges, &surfedges[first..first + count])?;
            if polygon.len() < 3 {
                continue;
            }
            let tw = i(&data[layer * 32..], 16);
            let th = i(&data[layer * 32..], 20);
            if tw <= 0 || th <= 0 {
                return Err("invalid texture dimensions".into());
            }
            let offset = i(a, 20);
            let lw = i(a, 36);
            let lh = i(a, 40);
            let mut packed = None;
            if offset >= 0 && !unlit[layer] {
                if lw < 0 || lh < 0 || lw > 1024 || lh > 1024 {
                    return Err("invalid face lightmap dimensions".into());
                }
                let w = lw as usize + 1;
                let h = lh as usize + 1;
                let pixels = lighting
                    .get(offset as usize..offset as usize + w * h * 4)
                    .ok_or("lightmap data bounds")?;
                if x + w + 2 > LIGHT_SIZE {
                    x = 2;
                    y += row + 2;
                    row = 0;
                }
                if y + h + 2 > LIGHT_SIZE {
                    return Err("lightmap atlas full".into());
                }
                // One-pixel duplicated border prevents neighbouring-face bilinear bleed.
                for py in 0..h + 2 {
                    for px in 0..w + 2 {
                        let src = ((py.saturating_sub(1).min(h - 1)) * w
                            + px.saturating_sub(1).min(w - 1))
                            * 4;
                        let dst = ((y + py - 1) * LIGHT_SIZE + x + px - 1) * 4;
                        let exp = pixels[src + 3] as i8 as i32;
                        // Saturation must scale RGB together, not bleach bright coloured luxels.
                        // Equivalent chromaticity-preserving ColorClamp for this LDR atlas.
                        let scale = (pixels[src..src + 3].iter().copied().max().unwrap() as f32
                            * 2f32.powi(exp)
                            / 255.)
                            .max(1.);
                        for c in 0..3 {
                            let linear = pixels[src + c] as f32 * 2f32.powi(exp) / 255. / scale;
                            lightmap[dst + c] = (linear.clamp(0., 1.) * 255.).round() as u8;
                        }
                        lightmap[dst + 3] = 255;
                    }
                }
                packed = Some((x, y));
                x += w + 2;
                row = row.max(h);
                lit += 1;
            }
            let project = |p: Vec3, off: usize| {
                p.dot(Vec3::new(f(tex, off), f(tex, off + 4), f(tex, off + 8))) + f(tex, off + 12)
            };
            let di = i16::from_le_bytes(a[12..14].try_into().unwrap());
            let geometry = if di >= 0 {
                if face_models[fi] != 0 {
                    return Err("displacement on moving brush unsupported".into());
                }
                let d = disp_info
                    .get(di as usize * 176..di as usize * 176 + 176)
                    .ok_or("face displacement index")?;
                if u(d, 36) != fi {
                    return Err("displacement map face mismatch".into());
                }
                let mesh = crate::displacement::tessellate(
                    &polygon.iter().copied().map(Vec3::from).collect::<Vec<_>>(),
                    d,
                    disp_verts,
                    disp_tris,
                )?;
                displacement_faces += 1;
                displacement_triangles += mesh.len() / 3;
                mesh.into_iter()
                    .map(|v| (v.position, v.base, v.normal))
                    .collect::<Vec<_>>()
            } else if di == -1 {
                (1..polygon.len() - 1)
                    .flat_map(|k| [polygon[0], polygon[k], polygon[k + 1]])
                    .map(|p| {
                        (
                            Vec3::from(p),
                            Vec3::from(p),
                            -(Vec3::from(polygon[1]) - Vec3::from(polygon[0]))
                                .cross(Vec3::from(polygon[2]) - Vec3::from(polygon[0]))
                                .normalize_or_zero(),
                        )
                    })
                    .collect()
            } else {
                return Err("negative displacement index".into());
            };
            for (pos, base, normal) in geometry {
                let light_uv = packed
                    .map(|(x, y)| {
                        [
                            (x as f32 + project(base, 32) - i(a, 28) as f32 + 0.5)
                                / LIGHT_SIZE as f32,
                            (y as f32 + project(base, 48) - i(a, 32) as f32 + 0.5)
                                / LIGHT_SIZE as f32,
                        ]
                    })
                    .unwrap_or([0.5 / LIGHT_SIZE as f32; 2]);
                out.push(MeshVertex {
                    position: (rotations[face_models[fi]] * pos).to_array(),
                    uv: [project(base, 0) / tw as f32, project(base, 16) / th as f32],
                    light_uv,
                    layer: layer as u32,
                    model: face_models[fi] as u32,
                    // Negative encoding keeps the vertex ABI stable for exports.
                    unlit: if translucent[layer] {
                        -1. - (u8::from(unlit[layer]) as f32)
                    } else if unlit[layer] {
                        1.
                    } else {
                        0.
                    },
                    normal: (rotations[face_models[fi]] * normal).to_array(),
                });
            }
        }
        let mut props = crate::props::static_props(b)?;
        let static_count = props.instances.len();
        let mut parents = vec![0usize; static_count];
        let mut initial_sequences = vec![None; static_count];
        let mut instance_names = vec![String::new(); static_count];
        let mut instance_solid = vec![false; static_count];
        for e in &entities {
            if ![
                "prop_dynamic",
                "prop_dynamic_override",
                "prop_physics",
                "weapon_portalgun",
            ]
            .contains(&e.get("classname").unwrap_or(""))
                || e.get("renderamt") == Some("0")
            {
                continue;
            }
            let weapon_model;
            let path = if e.get("classname") == Some("weapon_portalgun") {
                let script = fs.read("scripts/weapon_portalgun.txt")?;
                let tree = kv::parse(std::str::from_utf8(&script).map_err(|e| e.to_string())?)?;
                let fields = kv::child(&tree, "WeaponData").ok_or("weapon script block")?;
                weapon_model = kv::text(fields, "playermodel")
                    .ok_or("weapon script world model")?
                    .to_owned();
                Some(weapon_model.as_str())
            } else {
                e.get("model").filter(|p| p.ends_with(".mdl"))
            };
            let Some(path) = path else {
                continue;
            };
            if e.get("SetBodyGroup").unwrap_or("0") != "0" || e.get("body").unwrap_or("0") != "0" {
                warnings.push(format!("dynamic {path}: nonzero bodygroup unsupported"));
                continue;
            }
            let id = match props.models.iter().position(|p| p == path) {
                Some(id) => id,
                None => {
                    props.models.push(path.to_owned());
                    props.models.len() - 1
                }
            };
            let parent = if e.get("classname") == Some("weapon_portalgun") {
                let Some(model) = entity_runtime.entity_model(e.get("targetname").unwrap_or(""))
                else {
                    warnings.push("weapon world model capacity (256)".into());
                    continue;
                };
                model
            } else if e.get("classname") == Some("prop_physics") {
                entities
                    .iter()
                    .filter(|p| p.get("classname") == Some("prop_physics"))
                    .position(|p| std::ptr::eq(p, e))
                    .unwrap()
                    + models.len() / 48
            } else {
                entities
                    .iter()
                    .find(|p| {
                        p.get("targetname") == e.get("parentname") && e.get("parentname").is_some()
                    })
                    .map(|p| p.model())
                    .transpose()?
                    .flatten()
                    .unwrap_or(0)
            };
            instance_names.push(e.get("targetname").unwrap_or("").to_owned());
            instance_solid.push(e.get("solid").unwrap_or("6") != "0");
            parents.push(parent);
            initial_sequences.push(Some(e.get("DefaultAnim").unwrap_or("").to_owned()));
            props.instances.push(crate::props::StaticProp {
                model: u16::try_from(id).map_err(|_| "dynamic model count")?,
                origin: crate::entity::vector(e.get("origin").unwrap_or("0 0 0"))?,
                angles: crate::entity::vector(e.get("angles").unwrap_or("0 0 0"))?,
                skin: e
                    .get("skin")
                    .unwrap_or("0")
                    .parse()
                    .map_err(|_| "dynamic skin")?,
                solid: 0,
            });
        }
        let decoded = pool.install(|| {
            props
                .models
                .par_iter()
                .map(|path| {
                    let stem = path.trim_end_matches(".mdl");
                    crate::studio::decode(
                        &fs.read(path)?,
                        &fs.read(&format!("{stem}.vvd"))?,
                        &fs.read(&format!("{stem}.dx90.vtx"))?,
                    )
                })
                .collect::<Vec<_>>()
        });
        let mut prop_models = vec![];
        // Dynamic props start in their authored sequence, not the VVD bind pose.
        // Bind geometry can have a different basis even for a one-frame idle.
        let skeletons = props
            .models
            .iter()
            .map(|path| {
                fs.read(path)
                    .and_then(|mdl| crate::animation::Skeleton::decode(&mdl, None))
            })
            .collect::<Vec<_>>();
        let mut material_cache = std::collections::HashMap::<String, (usize, bool)>::new();
        for (path, decoded) in props.models.iter().zip(decoded) {
            let result = decoded.and_then(|model| {
                let mut layers = vec![];
                for candidates in &model.materials {
                    let mut found = None;
                    let mut error = String::new();
                    for name in candidates {
                        if let Some(layer) = material_cache.get(name) {
                            found = Some(*layer);
                            break;
                        }
                        match material(fs, name, 0) {
                            Ok(m) => {
                                if textures.len() >= 1024 {
                                    return Err("prop material layer capacity".into());
                                }
                                let layer = (textures.len(), m.unlit);
                                textures.push(resample(&m));
                                material_cache.insert(name.clone(), layer);
                                found = Some(layer);
                                break;
                            }
                            Err(e) => error = e,
                        }
                    }
                    layers.push(
                        found.ok_or_else(|| format!("prop material {candidates:?}: {error}"))?,
                    );
                }
                Ok((model, layers))
            });
            match result {
                Ok(m) => prop_models.push(Some(m)),
                Err(e) => {
                    warnings.push(format!("prop {path}: {e}"));
                    prop_models.push(None);
                }
            }
        }
        let leaves = crate::lump(b, 10, 32)?;
        let ambient_index = crate::lump(b, 52, 4)?;
        let ambient = crate::lump(b, 56, 28)?;
        if !ambient.is_empty()
            && (i(b, 8 + 56 * 16 + 8) != 1 || ambient_index.len() / 4 != leaves.len() / 32)
        {
            return Err("unsupported leaf ambient layout".into());
        }
        let mut static_props = 0;
        let mut dynamic_props = 0;
        let mut skipped_dynamic_props = 0;
        let mut skipped_props = 0;
        let mut prop_triangles = 0;
        let mut animations = vec![];
        for (instance, prop) in props.instances.into_iter().enumerate() {
            let Some((model, layers)) = &prop_models[prop.model as usize] else {
                if instance < static_count {
                    skipped_props += 1;
                } else {
                    skipped_dynamic_props += 1;
                }
                continue;
            };
            let Some(skin) = model.skins.get(prop.skin) else {
                warnings.push(format!(
                    "prop {} unsupported skin {}",
                    props.models[prop.model as usize], prop.skin
                ));
                if instance < static_count {
                    skipped_props += 1;
                } else {
                    skipped_dynamic_props += 1;
                }
                continue;
            };
            if out.len() + model.vertices.len() > 30_000_000 {
                return Err("scene prop vertex limit".into());
            }
            let rotation = crate::models::rotation(prop.angles);
            let posed;
            let vertices = if let Some(sequence) = &initial_sequences[instance] {
                match &skeletons[prop.model as usize] {
                    Ok(skeleton) => {
                        let sequence = if sequence.is_empty() {
                            skeleton.initial_sequence.as_deref().unwrap_or("")
                        } else {
                            sequence.as_str()
                        };
                        match skeleton.skin(&model.vertices, sequence, 0.) {
                            Ok(vertices) => {
                                posed = vertices;
                                &posed
                            }
                            Err(error) => {
                                warnings.push(format!(
                                    "initial pose {}: {error}; bind fallback",
                                    props.models[prop.model as usize]
                                ));
                                &model.vertices
                            }
                        }
                    }
                    Err(error) => {
                        warnings.push(format!(
                            "initial pose {}: {error}; bind fallback",
                            props.models[prop.model as usize]
                        ));
                        &model.vertices
                    }
                }
            } else {
                &model.vertices
            };
            // Nearest BSP LDR ambient sample, six directional cube lobes.
            let mut nearest = f32::INFINITY;
            let mut colours = [Vec3::ONE; 6];
            for (leaf, index) in leaves.chunks_exact(32).zip(ambient_index.chunks_exact(4)) {
                let short = |off| i16::from_le_bytes(leaf[off..off + 2].try_into().unwrap()) as f32;
                let min = Vec3::new(short(8), short(10), short(12));
                let max = Vec3::new(short(14), short(16), short(18));
                let first = u(index, 2);
                for sample in ambient
                    .get(first * 28..(first + u(index, 0)) * 28)
                    .ok_or("ambient sample range")?
                    .chunks_exact(28)
                {
                    let pos = min
                        + (max - min)
                            * Vec3::new(sample[24] as f32, sample[25] as f32, sample[26] as f32)
                            / 255.;
                    let distance = pos.distance_squared(prop.origin);
                    if distance < nearest {
                        nearest = distance;
                        for (side, cube) in sample[..24].chunks_exact(4).enumerate() {
                            colours[side] =
                                Vec3::new(cube[0] as f32, cube[1] as f32, cube[2] as f32)
                                    * 2f32.powi(cube[3] as i8 as i32);
                        }
                    }
                }
            }
            if !nearest.is_finite() {
                warnings.push(format!(
                    "prop {}: no ambient samples; unbaked white fallback",
                    props.models[prop.model as usize]
                ));
            }
            if x + 18 > LIGHT_SIZE {
                x = 2;
                y += row + 2;
                row = 0;
            }
            if y + 3 > LIGHT_SIZE {
                return Err("ambient lightmap atlas full".into());
            }
            for (side, mut colour) in colours.into_iter().enumerate() {
                colour /= colour.max_element().max(1.);
                for py in y - 1..=y + 1 {
                    for px in x + side * 3 - 1..=x + side * 3 + 1 {
                        let dst = (py * LIGHT_SIZE + px) * 4;
                        for c in 0..3 {
                            lightmap[dst + c] = (colour[c].clamp(0., 1.) * 255.).round() as u8;
                        }
                    }
                }
            }
            let prop_light_uv = [
                (x as f32 + 0.5) / LIGHT_SIZE as f32,
                (y as f32 + 0.5) / LIGHT_SIZE as f32,
            ];
            x += 18;
            row = row.max(1);
            let start = out.len();
            for v in vertices {
                let (layer, unlit) = layers[skin[v.material]];
                out.push(MeshVertex {
                    position: (rotation * v.position + prop.origin - origins[parents[instance]])
                        .to_array(),
                    normal: (rotation * v.normal).to_array(),
                    uv: v.uv,
                    light_uv: prop_light_uv,
                    layer: layer as u32,
                    model: parents[instance] as u32,
                    unlit: if unlit { 1. } else { 3. },
                });
            }
            if instance >= static_count {
                if let Ok(skeleton) = &skeletons[prop.model as usize] {
                    let initial = initial_sequences[instance]
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .or(skeleton.initial_sequence.as_deref())
                        .unwrap_or("");
                    if skeleton.clip(initial).is_ok() {
                        animations.push(AnimationInstance {
                            name: instance_names[instance].clone(),
                            range: start..out.len(),
                            solid: instance_solid[instance],
                            skeleton: skeleton.clone(),
                            initial: initial.to_owned(),
                            last_pose: (initial.to_owned(), 0.),
                            updated: false,
                            vertices: crate::animation::SkinMesh::new(&model.vertices),
                            rotation,
                            offset: prop.origin - origins[parents[instance]],
                        });
                    }
                }
            }
            if instance < static_count {
                static_props += 1;
            } else {
                dynamic_props += 1;
            }
            prop_triangles += model.vertices.len() / 3;
        }
        let physics = entities
            .iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .count();
        let base = models.len() / 48;
        let reserve = if physics == 0 {
            0
        } else {
            (256usize.saturating_sub(base + physics) / physics).min(8)
        };
        let original_end = out.len();
        for n in 0..physics {
            let prototype = out[..original_end]
                .iter()
                .filter(|v| v.model as usize == base + n)
                .copied()
                .collect::<Vec<_>>();
            prop_triangles += prototype.len() / 3 * reserve;
            for k in 0..reserve {
                for mut vertex in prototype.iter().copied() {
                    vertex.model = (base + physics + n * reserve + k) as u32;
                    out.push(vertex);
                }
            }
        }
        if out.is_empty() {
            return Err("no visible textured geometry".into());
        }
        let scene_durations = if entities
            .iter()
            .any(|e| e.get("classname") == Some("logic_choreographed_scene"))
        {
            match fs
                .read("scenes/scenes.image")
                .and_then(|b| crate::scenes::durations(&b))
            {
                Ok(durations) => durations,
                Err(error) => {
                    warnings.push(format!("scene completion timing unavailable: {error}"));
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };
        let mut pellet_template = vec![];
        if entities.iter().any(|e| e.get("classname") == Some("point_energy_ball_launcher")) {
            match Self::energy_ball(fs) {
                Ok((mut mesh, images)) => {
                    let offset = textures.len() as u32;
                    for v in &mut mesh { v.layer += offset; }
                    textures.extend(images);
                    pellet_template = mesh;
                }
                Err(e) => warnings.push(format!("original energy ball unavailable (not rendered): {e}")),
            }
        }
        Ok(Self {
            pellet_template,
            scene_durations,
            animations,
            static_props,
            dynamic_props,
            skipped_dynamic_props,
            skipped_props,
            prop_triangles,
            displacement_triangles,
            displacement_faces,
            vertices: out,
            textures,
            lightmap,
            filtered_faces: filtered,
            lightmapped_faces: lit,
            warnings,
            origins,
        })
    }
}
