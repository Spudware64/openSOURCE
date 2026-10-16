use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    portal::{Game, Surfaces},
    scene::Scene,
};
#[test]
fn actual_chamber00_thrown_cube_crosses_authored_pair() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.bind_animations(&scene);
    w.bind_scene(&scene);
    w.update_models(&r.origins, &r.visible);
    r.portal_events = Some(vec![]);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let mut p = Player::spawn(&w);
    for _ in 0..8100 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let [Some(src), Some(dst)] = w.portals else {
        panic!("authored pair not active")
    };
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let cube = r
        .origins
        .iter()
        .enumerate()
        .filter(|(i, _)| r.visible[*i] && *i >= 84)
        .min_by(|(_, a), (_, b)| {
            a.distance(Vec3::new(-800., -1056., 160.))
                .total_cmp(&b.distance(Vec3::new(-800., -1056., 160.)))
        })
        .unwrap()
        .0;
    // Inject only the approach pose; retain authored pair, actual mesh bounds and world collision.
    r.origins[cube] = src.center + src.normal * 40. - Vec3::Z * 32.;
    p.position = src.center + src.normal * 80. - Vec3::Z * 96.;
    p.yaw = (-src.normal).y.atan2((-src.normal).x);
    p.pitch = 0.;
    assert!(r.use_player(&p, &w), "pickup real cube slot {cube}");
    assert!(r.throw_held(&p));
    for _ in 0..20 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.visible[cube]);
    assert!(
        r.origins[cube].distance(dst.center) < 100.,
        "src {src:?} dst {dst:?} cube {:?}",
        r.origins[cube]
    );
}

#[test]
fn actual_chamber00_held_cube_persists_across_authored_pair() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.bind_animations(&scene);
    w.bind_scene(&scene);
    w.update_models(&r.origins, &r.visible);
    r.portal_events = Some(vec![]);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let mut p = Player::spawn(&w);
    for _ in 0..8100 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let [Some(src), Some(dst)] = w.portals else {
        panic!("authored pair not active")
    };
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let cube = r
        .origins
        .iter()
        .enumerate()
        .filter(|(i, _)| r.visible[*i] && *i >= 84)
        .min_by(|(_, a), (_, b)| {
            a.distance(Vec3::new(-800., -1056., 160.))
                .total_cmp(&b.distance(Vec3::new(-800., -1056., 160.)))
        })
        .unwrap()
        .0;
    // Inject only the approach pose; retain authored pair, actual mesh bounds and world collision.
    r.origins[cube] = src.center + src.normal * 40. - Vec3::Z * 32.;
    p.position = src.center + src.normal * 80. - Vec3::Z * 96.;
    p.yaw = (-src.normal).y.atan2((-src.normal).x);
    p.pitch = 0.;
    assert!(r.use_player(&p, &w), "pickup real cube slot {cube}");
    p.position = src.center + src.normal * 30. - Vec3::Z * 64.;
    for tick in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(
            r.origins[cube].distance(dst.center) < 100.,
            "tick {tick} held cube {:?}, source {:?} destination {:?}",
            r.origins[cube],
            src.center,
            dst.center
        );
    }
    p.position.z = src.center.z - p.height() * 0.5;
    p.velocity = -src.normal * 190.;
    // Cross with the real player controller, then keep the grab alive on
    // the destination side before dropping and checking collision ownership.
    for _ in 0..120 {
        p.step(&w, Vec3::X, false, 1. / 120.);
        r.tick(1. / 120., &mut p, &mut w);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert_eq!(
        p.portal_crossings, 1,
        "player must actually cross authored aperture"
    );
    p.velocity = Vec3::ZERO;
    for tick in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(
            r.origins[cube].distance(dst.center) < 120.,
            "post-player tick {tick}: {:?}",
            r.origins[cube]
        );
    }
    assert!(r.use_player(&p, &w), "drop held cube");
    r.tick(1. / 120., &mut p, &mut w);
    assert!(
        w.overlaps_cuboid(r.origins[cube] + Vec3::Z * 16., Vec3::splat(1.)),
        "dropped collider must follow destination model"
    );
    assert!(r.visible[cube]);
    assert!(
        r.origins[cube].distance(dst.center) < 100.,
        "src {src:?} dst {dst:?} cube {:?}",
        r.origins[cube]
    );
}
