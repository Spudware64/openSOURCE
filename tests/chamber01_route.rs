use portal_bsp_viewer::{
    assets::Assets,
    chamber01_route::Route,
    entity::Runtime,
    play::{Player, World},
    portal::{Game, Surfaces},
    scene::Scene,
};
#[test]
fn real_spawn_automatic_portal_pickup_owned_shot_return_and_exit_without_noclip() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    r.bind_animations(&s);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&s);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(w.spawn, w.yaw);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    assert_eq!(r.portal_inventory, [false, false]);
    assert!(g
        .fire_player(0, p.position, glam::Vec3::X, &r, &mut w)
        .is_err());
    let mut route = Route::default();
    while !route.tick(&mut p, &mut r, &mut w, &mut g).unwrap() {}
    assert_eq!(route.phase, 15);
    assert!(!p.noclip);
    assert!(p.grounded);
    assert_eq!(p.portal_crossings, 2);
    assert_eq!(r.portal_inventory, [true, false]);
    let gun = r.entity_model("portalgun").unwrap();
    assert!(!r.visible[gun]);
    let mut next = Runtime::new(&b).unwrap();
    next.enter_transition(&r, glam::Vec3::ZERO);
    assert_eq!(next.portal_inventory, [true, false]);
    println!("completed {} fixed ticks at {:?}", route.ticks, p.position);
}

#[test]
fn continuous_authored_spawn_to_chamber02() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    r.bind_animations(&s);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&s);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(w.spawn, w.yaw);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let mut route = Route::default();
    route.full = true;

    while !route.tick(&mut p, &mut r, &mut w, &mut g).unwrap() {}
    assert_eq!(
        r.change_level.as_ref().map(|q| q.map.as_str()),
        Some("testchmb_a_02")
    );
    assert_eq!(p.portal_crossings, 4);
    assert!(!p.noclip);
    let request = r.change_level.take().unwrap();
    let root = portal_bsp_viewer::session::GameRoot::new("../portal-original/portal", 4).unwrap();
    let mut next = root.prepare(&request, &p, true).unwrap();
    next.runtime
        .enter_transition(&r, next.player.position - p.position);
    next.world
        .update_models(&next.runtime.origins, &next.runtime.visible);
    for _ in 0..240 {
        next.player
            .step(&next.world, glam::Vec3::ZERO, false, 1. / 120.);
        next.runtime
            .tick(1. / 120., &mut next.player, &mut next.world);
    }
    assert!(next.player.grounded, "arrival {:?}", next.player.position);
    assert_eq!(next.runtime.portal_inventory, [true, false]);
    println!(
        "full route {} ticks; arrival {:?}",
        route.ticks, next.player.position
    );
}
