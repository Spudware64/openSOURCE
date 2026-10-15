use glam::Vec3;
use portal_bsp_viewer::{assets::Assets,entity::{parse_entities,Runtime},play::{World,Player},scene::Scene};
fn main(){
 let b=std::fs::read("../portal-original/portal/maps/testchmb_a_02.bsp").unwrap();
 let es=parse_entities(portal_bsp_viewer::lump(&b,0,1).unwrap()).unwrap();
 let fs=Assets::from_game("../portal-original/portal",Some(&b)).unwrap();let s=Scene::load(&b,&fs).unwrap();
 let mut r=Runtime::new(&b).unwrap();r.bind_animations(&s);let mut w=World::parse(&b).unwrap();w.bind_scene(&s);w.update_models(&r.origins,&r.visible);
 for name in ["box","box_2"] {let m=r.entity_model(name).unwrap();let (lo,hi)=s.vertices.iter().filter(|v|v.model as usize==m).fold((Vec3::splat(f32::INFINITY),Vec3::splat(f32::NEG_INFINITY)),|(lo,hi),v|{let p=Vec3::from(v.position);(lo.min(p),hi.max(p))});println!("{name} model {m} bounds {lo:?} {hi:?}");}
 for e in es.iter().filter(|e|e.get("classname")==Some("env_entity_maker")) {let pos=e.origin().unwrap();println!("maker {:?} {pos:?}",e.get("targetname"));for(m,bs)in w.model_brushes.iter().enumerate(){if !w.model_solid[m]{continue}for(j,b)in bs.iter().enumerate(){let local=pos-w.model_origins[m];if b.iter().all(|p|p.normal.dot(local)-p.distance-p.normal.abs().dot(Vec3::splat(20.))< -0.001){println!(" overlap model {m} brush {j} owners {:?} planes {:?}",es.iter().filter(|e|e.model().ok().flatten()==Some(m)).map(|e|&e.pairs).collect::<Vec<_>>(),b);}}}}
 let mut p=Player::new(w.spawn,w.yaw);for _ in 0..240{p.step(&w,Vec3::ZERO,false,1./120.);r.tick(1./120.,&mut p,&mut w);}println!("spawn {:?} grounded {} unknown {:?}",p.position,p.grounded,r.unknown_inputs);
}
