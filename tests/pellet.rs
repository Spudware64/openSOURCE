use glam::Vec3;
use opensource::{entity::{Runtime,parse_entities},play::{Player,World},portal::Portal};
fn fixture()->(Runtime,World,Player) {
 let e=parse_entities(br#"{"classname" "point_energy_ball_launcher" "targetname" "launcher" "origin" "50 0 0" "angles" "0 180 0" "minspeed" "200" "BallLifetime" "1" "ballrespawntime" "0.2"}"#).unwrap();
 (Runtime::from_entities(e,vec![]).unwrap(),World::from_brushes(vec![]),Player::new(Vec3::splat(10000.),0.))
}
#[test]
fn authored_launcher_enable_spawns_moving_pellet() {
 let b=std::fs::read("../portal-original/portal/maps/testchmb_a_04.bsp").unwrap();
 let mut r=Runtime::new(&b).unwrap(); let mut w=World::parse(&b).unwrap(); let mut p=Player::new(Vec3::splat(10000.),0.);
 r.tick(1./120.,&mut p,&mut w); assert!(r.pellets.is_empty());
 r.input("energy_ball_launcher_lvl5","Enable","");
 for _ in 0..240 {r.tick(1./120., &mut p,&mut w);}
 assert_eq!(r.pellets.len(),1);
 assert!((r.pellets[0].velocity.length()-200.).abs()<0.1);
 assert!(r.pellets[0].position.distance(Vec3::new(-331.51,318.,61.46))>100.,"authored ball must actually leave launcher");
}
#[test]
fn pellet_portal_preserves_speed_and_maps_direction() {
 let (mut r,mut w,mut p)=fixture();
 w.portals=[Some(Portal::new(Vec3::ZERO,Vec3::X,Vec3::Z)),Some(Portal::new(Vec3::new(1000.,0.,0.),Vec3::Y,Vec3::Z))];
 for _ in 0..40 {r.tick(1./120.,&mut p,&mut w);}
 assert!(r.pellets[0].position.x>990.);
 assert!(r.pellets[0].velocity.y>199.);
 assert!((r.pellets[0].velocity.length()-200.).abs()<0.01);
}
#[test]
fn expiry_waits_before_respawn() {
 let (mut r,mut w,mut p)=fixture();
 for _ in 0..121 {r.tick(1./120.,&mut p,&mut w);}
 assert!(r.pellets.is_empty());
 for _ in 0..26 {r.tick(1./120.,&mut p,&mut w);}
 assert_eq!(r.pellets.len(),1);
}
#[test]
fn swept_pellet_bounces_without_speed_loss() {
 let (mut r,_,mut p)=fixture();
 let mut w=World::from_brushes(vec![vec![opensource::play::Plane{normal:Vec3::X,distance:0.}]]);
 for _ in 0..40 {r.tick(1./120.,&mut p,&mut w);}
 assert!(r.pellets[0].velocity.x>199.);
 assert!(r.pellets[0].position.x>12.);
}
#[test]
fn real_catcher_fires_authored_platform_outputs() {
 let b=std::fs::read("../portal-original/portal/maps/testchmb_a_04.bsp").unwrap();
 let mut r=Runtime::new(&b).unwrap(); let mut w=World::parse(&b).unwrap();let mut p=Player::new(Vec3::splat(10000.),0.);
 r.input("energy_ball_launcher_lvl5","Enable","");
 for _ in 0..3 {r.tick(1./120.,&mut p,&mut w);}
 // Isolated detector regression: inject a real launched pellet into the authored trigger approach, not a solved map route.
 let entities=parse_entities(opensource::lump(&b,0,1).unwrap()).unwrap();
 let center=entities.iter().find(|e|e.get("targetname")==Some("ball_trap_10")).unwrap().origin().unwrap();
 r.pellets[0].position=center-Vec3::Y*64.;r.pellets[0].velocity=Vec3::Y*200.;
 for _ in 0..60 {r.tick(1./120.,&mut p,&mut w);}
 assert!(!r.is_enabled("ball_trap_10"));
 assert!(!r.is_enabled("energy_ball_launcher_lvl5"));
 assert!(r.pellets.is_empty());
}
