use glam::Vec3;
use opensource::{
    entity::Runtime,
    play::{Player, World},
};
#[test]
fn actual_weapon_pickup_fires_its_exit_outputs_once() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(
        Vec3::new(96., 200., -119.96875),
        -std::f32::consts::FRAC_PI_2,
    );
    p.pitch = -0.2;
    // Nearby player touches/uses the real gun, not a smoke-issued exit-door input.
    assert!(r.use_player(&p, &w), "real pedestal gun must be pickable");
    for _ in 0..120 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[10].x > -190.,
        "door {:?}, player {:?} overlaps {} unknown {:?}",
        r.origins[10],
        p.position,
        w.overlaps(p.position, p.height()),
        r.unknown_inputs
    );
    assert!(!r.is_enabled("fire_rotating_portalgun_tca01"));
}
