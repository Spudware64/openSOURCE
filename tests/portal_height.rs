use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    play::{Player, World},
    portal::{Portal, Surfaces},
};
#[test]
fn actual_east_placed_portal_jump_cannot_leak_through_without_crossing() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let mut w = World::parse(&b).unwrap();
    w.portals = [
        Some(Portal::new(Vec3::new(320., 200., -72.), -Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(-384., 288., 52.), Vec3::X, Vec3::Z)),
    ];
    let mut p = Player::new(Vec3::new(284.75684, 200.0118, -127.96875), 0.);
    for tick in 0..120 {
        p.step(&w, Vec3::X, tick == 0, 1. / 120.);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert_eq!(
        p.portal_crossings, 1,
        "portal aperture leaked without teleport: {:?}",
        p.position
    );
}
#[test]
fn actual_north_automatic_portal_jump_cannot_leak_through_without_crossing() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Surfaces::load(&b, &fs).unwrap();
    let mut w = World::parse(&b).unwrap();
    w.portals = [
        Some(
            s.place(
                Vec3::new(95.5556, 172.9549, -72.9827),
                Vec3::new(0., 1., 0.00872655),
            )
            .unwrap(),
        ),
        Some(Portal::new(Vec3::new(-384., 288., 52.), Vec3::X, Vec3::Z)),
    ];
    let mut p = Player::new(
        Vec3::new(96., 544., -127.96875),
        std::f32::consts::FRAC_PI_2,
    );
    for tick in 0..240 {
        p.step(&w, Vec3::X, tick % 120 == 0, 1. / 120.);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert_eq!(
        p.portal_crossings, 1,
        "portal aperture leaked without teleport: {:?}",
        p.position
    );
    assert!(
        p.position.x < -300. && p.position.z > -20.,
        "{:?}",
        p.position
    );
}
