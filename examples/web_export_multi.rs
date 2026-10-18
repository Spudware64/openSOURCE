use opensource::{assets::Assets, play::World, scene::Scene};
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
 let args:Vec<String>=std::env::args().collect(); let map=&args[1]; let outp=&args[2];
 let b=std::fs::read(format!("../portal-original/portal/maps/{map}.bsp"))?; let fs=Assets::from_game("../portal-original/portal",Some(&b))?; let s=Scene::load(&b,&fs)?; let w=World::parse(&b)?; let mut out=std::fs::File::create(outp)?;
 for n in [s.vertices.len(),s.textures.len(),w.brushes.len()] {out.write_all(&(n as u32).to_le_bytes())?;} for f in [w.spawn.x,w.spawn.y,w.spawn.z,w.yaw]{out.write_all(&f.to_le_bytes())?;}
 for v in &s.vertices {let o=s.origins.get(v.model as usize).copied().unwrap_or_default(); for f in [v.position[0]+o.x,v.position[1]+o.y,v.position[2]+o.z,v.uv[0],v.uv[1],v.light_uv[0],v.light_uv[1],v.layer as f32,v.unlit,v.normal[0],v.normal[1],v.normal[2]]{out.write_all(&f.to_le_bytes())?;}}
 for t in &s.textures {out.write_all(t)?;} out.write_all(&s.lightmap)?; for brush in &w.brushes {out.write_all(&(brush.len() as u32).to_le_bytes())?;for p in brush{for f in [p.normal.x,p.normal.y,p.normal.z,p.distance]{out.write_all(&f.to_le_bytes())?;}}}
 eprintln!("{map}: vertices={} textures={} brushes={} spawn={:?}",s.vertices.len(),s.textures.len(),w.brushes.len(),w.spawn); Ok(())
}