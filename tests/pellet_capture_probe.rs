//! Continuous diagnostic route using original chamber04 launcher and receiver.
//! Portals are explicitly staged, not legal player placements or a chamber solve.
use glam::Vec3;
use portal_bsp_viewer::{entity::Runtime, play::{Player, World}, portal::Portal};
#[test]
fn staged_pair_continuously_routes_authored_launcher_to_authored_receiver() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_04.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::splat(10000.), 0.);
    w.portals = [
        Some(Portal::new(Vec3::new(-100.,318.,61.46),-Vec3::X,Vec3::Z)),
        Some(Portal::new(Vec3::new(-1360.,878.,-56.),Vec3::X,Vec3::Z)),
    ];
    r.input("energy_ball_launcher_lvl5", "Enable", "");
    let mut previous = None;
    let mut crossings = 0;
    for _ in 0..360 {
        // Never inject, reposition, or steer any launched pellet.
        r.tick(1./120., &mut p, &mut w);
        if let Some(ball) = r.pellets.first() {
            assert!((ball.velocity.length()-200.).abs()<0.01);
            if previous.is_some_and(|x| x > -1000.) && ball.position.x < -1000. {
                crossings += 1;
                assert!((ball.position.y-878.).abs()<0.01);
            }
            previous = Some(ball.position.x);
        }
        if !r.is_enabled("ball_trap_10") {
            assert_eq!(crossings, 1);
            assert!(r.pellets.is_empty());
            // This is an authored OnTrigger effect, not directly driven by the test.
            assert!(!r.is_enabled("energy_ball_launcher_lvl5"));
            return;
        }
    }
    panic!("continuous pellet failed to activate original receiver");
}
