use glam::Vec3;
use portal_bsp_viewer::{
    entity::Runtime,
    play::{Player, World},
};
#[test]
fn authored_middle_elevator_carries_player_and_passes_tracks() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let fs = portal_bsp_viewer::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = portal_bsp_viewer::scene::Scene::load(&b, &fs).unwrap();
    w.bind_scene(&scene);
    r.bind_animations(&scene);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(Vec3::new(-1472., -864., 200.), 0.);
    for _ in 0..120 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(p.grounded, "elevator floor {:?}", p.position);
    let start = p.position;
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert_eq!(r.origins[37].z, 214., "authored five-second delay");
    for _ in 0..1440 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(
        r.origins[37].z > 780.,
        "train {:?}; unknown {:?}",
        r.origins[37],
        r.unknown_inputs
    );
    assert!(
        (p.position.z - start.z - 576.).abs() < 1.,
        "rider start {start:?} end {:?}",
        p.position
    );
    assert!(p.grounded);
    assert!(!w.overlaps(p.position, p.height()));
    let stopped = r.origins[37];
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert_eq!(r.origins[37], stopped);
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert!(
        p.position.x > -1200.,
        "must dismount into upper corridor: {:?}",
        p.position
    );
    assert!(
        p.position.z > 700. && p.grounded,
        "upper landing {:?}",
        p.position
    );
    assert_eq!(r.origins[37], stopped);
    println!(
        "real elevator: start {start:?}, train {stopped:?}, upper corridor {:?}; unknown {:?}",
        p.position, r.unknown_inputs
    );
}
