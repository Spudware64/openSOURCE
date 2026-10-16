use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    portal::Game,
    scene::Scene,
};
#[test]
fn authored_second_room_open_gate_crosses_fitted_blue2() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut scene = Scene::load(&b, &fs).unwrap();
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
    assert!(pair[0].unwrap().normal.dot(Vec3::X) > 0.999);
    let closed_door = r.entity_transform("door_left_00").unwrap();
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
    assert_ne!(
        r.entity_transform("door_left_00").unwrap(),
        closed_door,
        "real cube must open actual door"
    );
    for _ in 0..2400 {
        if w.portals[0].is_some_and(|q| q.center.distance(pair[0].unwrap().center) < 0.1) {
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
    if let Ok(path) = std::env::var("BLUE2_CAPTURE") {
        scene.origins = r.origins.clone();
        let pixels = portal_bsp_viewer::render_portal_scene(
            &scene,
            1024,
            768,
            Some(([-1088., -520., 640.], [-1088., -632., 640.])),
            w.portals,
        )
        .unwrap();
        let mut png = png::Encoder::new(std::fs::File::create(path).unwrap(), 1024, 768);
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    assert_eq!(
        w.portals[0].unwrap().center,
        pair[0].unwrap().center,
        "still blue2 at crossing"
    );
    assert!(
        p.portal_crossings > 0,
        "open gate must permit blue2 crossing"
    );
    assert!(p.position.x < -1200.);
}

#[test]
fn authored_placement_uses_supporting_face_and_preserves_aligned_portals() {
    use portal_bsp_viewer::portal::{Portal, Surfaces};
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let surfaces = Surfaces::load(&b, &fs).unwrap();
    let raw = Portal::new(Vec3::new(-1344., -224., 631.86), -Vec3::Y, Vec3::Z);
    let face = surfaces
        .faces
        .iter()
        .find(|s| {
            s.material == "concrete/concrete_modular_wall001e"
                && s.normal.dot(Vec3::X) > 0.999
                && s.points.iter().all(|p| p.x == -1344.)
                && s.points.iter().any(|p| p.y == -320.)
        })
        .unwrap();
    assert!(face.allowed);
    let fitted = surfaces.fit_authored(raw).unwrap();
    assert_eq!(fitted.center, raw.center);
    assert!(fitted.normal.dot(Vec3::X) > 0.999);
    for (center, normal) in [
        (Vec3::new(-1376., -384., 628.), -Vec3::Y),
        (Vec3::new(-800., -384., 628.), -Vec3::Y),
        (Vec3::new(-1088., -632., 628.), Vec3::Y),
    ] {
        let p = Portal::new(center, normal, Vec3::Z);
        let fitted = surfaces.fit_authored(p).unwrap();
        assert_eq!(fitted.center, p.center);
        assert!(fitted.normal.dot(p.normal) > 0.999);
    }
}
