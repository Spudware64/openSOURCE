use glam::Vec3;
use portal_bsp_viewer::{assets::Assets,entity::Runtime,play::{Player,World},scene::Scene};
#[test]
fn authored_mapspawn_force_spawns_all_three_bodies_without_space_check_flag(){
 let b=std::fs::read("../portal-original/portal/maps/testchmb_a_02.bsp").unwrap();let fs=Assets::from_game("../portal-original/portal",Some(&b)).unwrap();let s=Scene::load(&b,&fs).unwrap();let mut r=Runtime::new(&b).unwrap();r.bind_animations(&s);let mut w=World::parse(&b).unwrap();w.bind_scene(&s);w.update_models(&r.origins,&r.visible);let mut p=Player::new(w.spawn,w.yaw);
 r.tick(1./120.,&mut p,&mut w);
 assert!(!r.unknown_inputs.contains("blocked physics spawn"),"{:?}",r.unknown_inputs);
 let bodies=r.origins.iter().zip(&r.visible).filter(|(v,on)|**on && v.z>400. && v.x>650.).map(|(v,_)|*v).collect::<Vec<_>>();
 assert!(bodies.iter().any(|v|v.distance(Vec3::new(731.024,579.027,712.))<1.));assert!(bodies.iter().any(|v|v.distance(Vec3::new(970.006,816.835,456.))<1.));
}
