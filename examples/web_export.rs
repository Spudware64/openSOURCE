use portal_bsp_viewer::{assets::Assets, play::World, scene::Scene};
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp")?;
    let fs = Assets::from_game("../portal-original/portal", Some(&b))?;
    let s = Scene::load(&b, &fs)?;
    let w = World::parse(&b)?;
    let mut out = std::fs::File::create("artifacts/web/scene.bin")?;
    for n in [s.vertices.len(), s.textures.len(), w.brushes.len()] {
        out.write_all(&(n as u32).to_le_bytes())?;
    }
    for f in [w.spawn.x, w.spawn.y, w.spawn.z, w.yaw] {
        out.write_all(&f.to_le_bytes())?;
    }
    for v in &s.vertices {
        let o = s.origins.get(v.model as usize).copied().unwrap_or_default();
        for f in [
            v.position[0] + o.x,
            v.position[1] + o.y,
            v.position[2] + o.z,
            v.uv[0],
            v.uv[1],
            v.light_uv[0],
            v.light_uv[1],
            v.layer as f32,
            v.unlit,
            v.normal[0],
            v.normal[1],
            v.normal[2],
        ] {
            out.write_all(&f.to_le_bytes())?;
        }
    }
    for t in &s.textures {
        out.write_all(t)?;
    }
    out.write_all(&s.lightmap)?;
    for brush in &w.brushes {
        out.write_all(&(brush.len() as u32).to_le_bytes())?;
        for p in brush {
            for f in [p.normal.x, p.normal.y, p.normal.z, p.distance] {
                out.write_all(&f.to_le_bytes())?;
            }
        }
    }
    println!(
        "vertices={} textures={} brushes={} spawn={:?} warnings={:?}",
        s.vertices.len(),
        s.textures.len(),
        w.brushes.len(),
        w.spawn,
        s.warnings
    );
    Ok(())
}
