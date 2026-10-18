use glam::Vec3;
use opensource::{
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
    // Reproduce the original running jump at the aperture. Ground input now
    // accelerates rather than granting full speed on the first jump tick.
    p.velocity = Vec3::X * 190.;
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

#[test]
fn actual_floor_partially_inserted_hull_cannot_slide_out_through_rim() {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let surfaces = Surfaces::load(&bytes, &fs).unwrap();
    for yaw in [0_f32, 45., 90.] {
        let mut w = World::parse(&bytes).unwrap();
        let placed = surfaces
            .place(Vec3::new(-416., -320., 178.), -Vec3::Z)
            .unwrap();
        let floor = Portal::new(
            placed.center,
            placed.normal,
            Vec3::new(yaw.to_radians().cos(), yaw.to_radians().sin(), 0.),
        );
        let wall = surfaces
            .place(Vec3::new(-1088., -600., 640.), -Vec3::Y)
            .unwrap();
        w.portals = [Some(floor), Some(wall)];
        let start = floor.center - Vec3::Z * 20.; // feet inside backing floor, centre still in front
        let end = start + floor.right() * 60.;
        let (t, n) = w.trace_height(start, end, 72.);
        assert!(t < 1., "yaw {yaw}: partially inserted hull escaped rim");
        assert!(n.dot(floor.right()) < -0.5, "yaw {yaw}: {n:?}");
        let mut p = Player::new(start, 0.);
        p.velocity = floor.right() * 190. - Vec3::Z * 80.;
        for _ in 0..120 {
            p.step(&w, Vec3::ZERO, false, 1. / 120.);
            if p.portal_crossings > 0 {
                break;
            }
            assert!(
                p.position.z + 36. >= floor.center.z,
                "yaw {yaw}: leaked {:?}",
                p.position
            );
        }
        assert_eq!(p.portal_crossings, 1, "yaw {yaw}: stuck {:?}", p.position);
    }
}

#[test]
fn actual_floor_rejected_crossing_keeps_inserted_hull_on_source_side() {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let surfaces = Surfaces::load(&bytes, &fs).unwrap();
    let mut w = World::parse(&bytes).unwrap();
    let floor = surfaces
        .place(Vec3::new(-416., -320., 178.), -Vec3::Z)
        .unwrap();
    // The linked destination is buried in the real backing floor: transfer
    // must be rejected even if the source hull was already partly inserted.
    let blocked = Portal::new(floor.center - Vec3::Z * 4., Vec3::X, Vec3::Z);
    w.portals = [Some(floor), Some(blocked)];
    let mut p = Player::new(floor.center - Vec3::Z * 20., 0.);
    p.velocity = -Vec3::Z * 180.;
    for _ in 0..120 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
        assert_eq!(p.portal_crossings, 0);
        assert!(p.position.z + 36. > floor.center.z, "{:?}", p.position);
    }
    // An ordinary hull outside the opening still lands on the actual floor.
    let start = floor.center + floor.right() * 90. + Vec3::Z * 20.;
    let (t, n) = w.trace_height(start, start - Vec3::Z * 40., 72.);
    assert!(t < 1. && n.z > 0.7, "{t} {n:?}");
}
