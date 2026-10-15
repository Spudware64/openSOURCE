use glam::Vec3;
use portal_bsp_viewer::play::*;
#[test]
fn scripted_window_smoke_walks_jumps_and_lands() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let w = World::parse(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    let mut max_z = p.position.z;
    for frame in 0..600 {
        smoke_tick(frame, &mut p, &w);
        max_z = max_z.max(p.position.z);
    }
    assert!(p.position.distance(w.spawn) > 20.);
    assert!(p.position.distance(w.spawn) < 500.);
    assert!(max_z > w.spawn.z + 30.);
    assert!(p.grounded);
    assert!(!p.noclip);
    assert!(p.velocity.z.abs() < 1.);
    eprintln!(
        "smoke end {:?} grounded {} jump apex {}",
        p.position, p.grounded, max_z
    );
    let _ = Vec3::ZERO;
}
