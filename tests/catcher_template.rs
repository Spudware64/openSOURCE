use glam::Vec3;
use opensource::{assets::Assets,entity::parse_entities,scene::Scene};
#[test]
fn map03_startup_receiver_uses_maker_transform() {
 let b=std::fs::read("../portal-original/portal/maps/testchmb_a_03.bsp").unwrap();
 let e=parse_entities(opensource::lump(&b,0,1).unwrap()).unwrap();
 let housing=e.iter().find(|e|e.get("targetname")==Some("ball_trap_10_housing")).unwrap();
 assert!(housing.origin().unwrap().distance(Vec3::new(-0.4186,-127.568,-118.4406))<0.05,"receiver must be at startup maker, not remote template storage: {:?}",housing.origin());
 let fs=Assets::from_game("../portal-original/portal",Some(&b)).unwrap();
 let scene=Scene::load(&b,&fs).unwrap();
 let a=scene.animations.iter().find(|a|a.name=="ball_trap_10_housing").unwrap();
 assert!(!a.range.is_empty());
 let (lo,hi)=scene.vertices[a.range.clone()].iter().fold((Vec3::splat(f32::INFINITY),Vec3::splat(f32::NEG_INFINITY)),|(lo,hi),v|{let p=Vec3::from(v.position);(lo.min(p),hi.max(p))});
 eprintln!("receiver initial={} vertices={} bounds={lo:?}..{hi:?}",a.initial,a.range.len());
 assert!(lo.x>-150. && hi.x<150. && lo.y>-250. && hi.y<0.);
 let mut runtime=opensource::entity::Runtime::new(&b).unwrap();
 runtime.bind_animations(&scene);
 let mut world=opensource::play::World::parse(&b).unwrap();
 let mut player=opensource::play::Player::new(Vec3::splat(10000.),0.);
 for _ in 0..3 {runtime.tick(1./120.,&mut player,&mut world);}
 assert!(!runtime.pellets.is_empty());
 let center=e.iter().find(|e|e.get("targetname")==Some("ball_trap_10")).unwrap().origin().unwrap();
 // Detector/I/O isolation, not a chamber solve or normal-input route.
 runtime.pellets[0].position=center+Vec3::Z*64.;
 runtime.pellets[0].velocity=-Vec3::Z*200.;
 for _ in 0..300 {runtime.tick(1./120.,&mut player,&mut world);}
 assert!(!runtime.is_enabled("ball_trap_10"));
 assert!(!runtime.is_enabled("ceiling_energy_ball_launcher_lvl4"));
 assert!(runtime.pellets.is_empty());
 assert_eq!(runtime.animation_pose("ball_trap_10_housing").unwrap().0,"closed");
}
