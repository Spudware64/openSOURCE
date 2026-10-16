use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    portal::Game,
    scene::Scene,
};
#[test]
fn authored_second_room_open_gate_retains_blue2_sidewall_blocker() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&scene);
    r.bind_animations(&scene);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(portal_bsp_viewer::portal::Surfaces::load(&b, &fs).unwrap());
    let mut p = Player::spawn(&w);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    p.position = Vec3::new(-1088., -512., 576.03125);
    for _ in 0..1080 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let pair = w.portals;
    eprintln!("pair {pair:?}");
    let closed_door = r.entity_transform("door_left_00").unwrap();
    p.yaw = -std::f32::consts::FRAC_PI_2;
    for _ in 0..100 {
        p.step(&w, Vec3::X, false, 1. / 120.);
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    assert_eq!(p.portal_crossings, 0, "closed gate must reject exit");
    eprintln!("closed {:?}", p.position);
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let cube = r.entity_model("box").unwrap();
    assert!(r.visible[cube]);
    // Stage only approach poses, using a real maker-spawned cube and production carry/drop.
    p.position = Vec3::new(-1376., -484., 576.03125);
    p.yaw = -std::f32::consts::FRAC_PI_2;
    p.pitch = -0.6;
    r.origins[cube] = Vec3::new(-1376., -534., 600.);
    r.tick(1. / 120., &mut p, &mut w);
    assert!(r.use_player(&p, &w), "pickup cube {cube}");
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    p.position.y -= 32.;
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    eprintln!("held {:?}", r.origins[cube]);
    p.pitch = -1.1;
    p.position.y = -544.;
    p.velocity = Vec3::ZERO;
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    assert!(r.use_player(&p, &w), "drop cube");
    p.position = Vec3::new(-1088., -512., 576.03125);
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    eprintln!(
        "dropped {:?} actual {:?}",
        r.origins[cube],
        r.rapier_collider_center(cube)
    );
    eprintln!("box model {:?}", r.entity_model("box"));
    for (i, brush) in w.brushes.iter().enumerate() {
        if brush
            .iter()
            .all(|q| q.normal.dot(r.origins[cube]) - q.distance < 17.)
        {
            eprintln!("near cube brush {i} {brush:?}");
        }
    }
    assert_ne!(
        r.entity_transform("door_left_00").unwrap(),
        closed_door,
        "real cube must open actual door"
    );
    for _ in 0..2400 {
        if w.portals[0].is_some_and(|q| q.center.distance(Vec3::new(-1344., -224., 631.86)) < 0.1) {
            break;
        }
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    assert_eq!(
        w.portals.map(|q| q.map(|v| v.center)),
        pair.map(|q| q.map(|v| v.center)),
        "must test exact blue2, not a cycling easier exit"
    );
    p.position = Vec3::new(-1088., -512., 576.03125);
    p.yaw = -std::f32::consts::FRAC_PI_2;
    p.pitch = 0.;
    p.velocity = Vec3::ZERO;
    for _ in 0..100 {
        p.step(&w, Vec3::X, false, 1. / 120.);
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    eprintln!(
        "open {:?} crossings {} pair {:?}",
        p.position, p.portal_crossings, w.portals
    );
    for (i, b) in w.brushes.iter().enumerate() {
        let c = Vec3::new(-1344., -240.1, 615.89);
        let e = Vec3::new(16., 16., 36.);
        if b.iter()
            .all(|q| q.normal.dot(c) - q.distance - q.normal.abs().dot(e) < -0.01)
        {
            eprintln!("exit overlap {i} {b:?}");
        }
    }
    // Known defect characterization, not expected Portal gameplay: the static
    // perpendicular sidewall remains after the real pressure plate opens the door.
    assert_eq!(
        p.portal_crossings, 0,
        "update this characterization when blue2 is fixed"
    );
    assert!(
        p.position
            .distance(Vec3::new(-1088., -615.96875, 576.03125))
            < 0.1
    );
}
