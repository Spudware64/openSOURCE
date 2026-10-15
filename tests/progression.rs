use glam::Vec3;
use portal_bsp_viewer::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};
fn map(name: &str) -> Vec<u8> {
    std::fs::read(format!("../portal-original/portal/maps/{name}.bsp")).unwrap()
}
#[test]
fn authored_changelevel_requires_input_and_preserves_landmark_offset() {
    let b = map("testchmb_a_00");
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(
        Vec3::new(-312., -256.82, 948.56) + Vec3::new(5., 7., -40.),
        0.7,
    );
    r.tick(0.01, &mut p, &mut w);
    assert!(r.change_level.is_none(), "spawnflags 2 disables touch");
    r.input("a00-a01_change_level_a01_02", "ChangeLevel", "");
    r.tick(0.01, &mut p, &mut w);
    let request = r
        .change_level
        .take()
        .expect("authored input must request transition");
    assert_eq!(request.map, "testchmb_a_01");
    let next = map(&request.map);
    let entities = parse_entities(portal_bsp_viewer::lump(&next, 0, 1).unwrap()).unwrap();
    let pos = request.arrival(&entities, p.position).unwrap();
    assert!(pos.distance(Vec3::new(909., 70.18, -243.44)) < 0.01);
}
#[test]
fn retained_game_root_prepares_fresh_map_and_resets_runtime() {
    let b = map("testchmb_a_00");
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    r.input("a00-a01_change_level_a01_02", "ChangeLevel", "");
    r.tick(0.01, &mut p, &mut w);
    let root = portal_bsp_viewer::session::GameRoot::new("../portal-original/portal", 2).unwrap();
    let next = root
        .prepare(&r.change_level.take().unwrap(), &p, true)
        .unwrap();
    assert!(next.scene.vertices.len() > 1000);
    assert!(next.runtime.change_level.is_none());
    assert_eq!(next.world.spawn, next.player.position);
    assert!(next.runtime.portal_events.is_some());
    assert!(next.game.portals.iter().all(Option::is_none));
}
#[test]
fn cleanser_removes_real_cube_collision_and_fires_authored_output_once() {
    let b = map("testchmb_a_00");
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.portal_events = Some(vec![]);
    let es = parse_entities(portal_bsp_viewer::lump(&b, 0, 1).unwrap()).unwrap();
    let m = portal_bsp_viewer::lump(&b, 14, 48).unwrap().len() / 48
        + es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .position(|e| e.get("targetname") == Some("box"))
            .unwrap();
    r.origins[m] = Vec3::new(-1264., -864., 180.);
    let mut p = Player::new(w.spawn, w.yaw);
    r.tick(1. / 120., &mut p, &mut w);
    assert!(!r.visible[m]);
    assert!(!r.use_player(&Player::new(Vec3::new(-1320., -864., 140.), 0.), &w));
    assert!(
        !r.is_enabled("show_drop_hudhint"),
        "authored OnDissolveBox must disable the HUD hint"
    );
    let mut game = portal_bsp_viewer::portal::Game::load(&b).unwrap();
    game.place(
        0,
        portal_bsp_viewer::portal::Portal::new(Vec3::ZERO, Vec3::X, Vec3::Z),
    );
    p.position = Vec3::new(-1264., -864., 144.);
    r.tick(1. / 120., &mut p, &mut w);
    game.sync_runtime(&mut r, &mut w);
    assert!(game.portals.iter().all(Option::is_none));
    assert!(w.portals.iter().all(Option::is_none));
    assert!(!r.cleanse_portals);
}
#[test]
fn final_elevator_authored_touch_track_output_requests_next_map() {
    let b = map("testchmb_a_00");
    let fs = portal_bsp_viewer::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = portal_bsp_viewer::scene::Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.bind_animations(&scene);
    w.bind_scene(&scene);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(Vec3::new(-384., -256., 600.), 0.);
    for _ in 0..3600 {
        r.tick(1. / 120., &mut p, &mut w);
        if r.change_level.is_some() {
            break;
        }
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert_eq!(
        r.change_level.as_ref().map(|c| c.map.as_str()),
        Some("testchmb_a_01"),
        "player {:?} unknown {:?}",
        p.position,
        r.unknown_inputs
    );
    assert!(
        p.position.z > 1100.,
        "must ride real final elevator, not issue direct ChangeLevel"
    );
    println!(
        "authored final elevator ChangeLevel player {:?}",
        p.position
    );
    let request = r.change_level.take().unwrap();
    let root = portal_bsp_viewer::session::GameRoot::new("../portal-original/portal", 2).unwrap();
    let mut next = root.prepare(&request, &p, true).unwrap();
    next.runtime
        .enter_transition(&r, next.player.position - p.position);
    next.world
        .update_models(&next.runtime.origins, &next.runtime.visible);
    for _ in 0..600 {
        next.runtime
            .tick(1. / 120., &mut next.player, &mut next.world);
        next.player.step(&next.world, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(
        next.player.position.z > 0. && next.player.grounded,
        "destination global elevator must catch rider {:?}",
        next.player.position
    );
    assert!(
        next.runtime
            .animation_pose("a00-a01_elevator_door_model")
            .is_some_and(|(n, _)| n == "open"),
        "OnMapTransition must open arrival elevator"
    );
    next.player.yaw = std::f32::consts::PI;
    for _ in 0..240 {
        next.runtime
            .tick(1. / 120., &mut next.player, &mut next.world);
        next.player.step(&next.world, Vec3::X, false, 1. / 120.);
    }
    assert!(
        next.player.position.x < 600. && next.player.position.z > 0.,
        "must walk off arrival elevator into next chamber {:?}",
        next.player.position
    );
}
#[test]
fn all_supplied_map_links_and_cleanser_brush_bounds_are_exercised() {
    let mut maps = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir("../portal-original/portal/maps").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|x| x == "bsp") {
            let bytes = std::fs::read(&path).unwrap();
            maps.insert(
                path.file_stem().unwrap().to_str().unwrap().to_owned(),
                bytes,
            );
        }
    }
    let mut links = 0;
    let mut cleansers = 0;
    for (name, bytes) in &maps {
        let es = parse_entities(portal_bsp_viewer::lump(bytes, 0, 1).unwrap()).unwrap();
        let bounds: Vec<_> = portal_bsp_viewer::lump(bytes, 14, 48)
            .unwrap()
            .chunks_exact(48)
            .map(|b| {
                let f = |o| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
                (Vec3::new(f(0), f(4), f(8)), Vec3::new(f(12), f(16), f(20)))
            })
            .collect();
        for e in &es {
            if e.get("classname") == Some("trigger_changelevel") {
                let target = e.get("map").unwrap();
                let landmark = e.get("landmark").unwrap();
                let next =
                    parse_entities(portal_bsp_viewer::lump(&maps[target], 0, 1).unwrap()).unwrap();
                let source = es
                    .iter()
                    .find(|x| {
                        x.get("classname") == Some("info_landmark")
                            && x.get("targetname") == Some(landmark)
                    })
                    .unwrap()
                    .origin()
                    .unwrap();
                let req = portal_bsp_viewer::entity::ChangeLevel {
                    map: target.into(),
                    landmark: landmark.into(),
                    source,
                };
                assert!(req.arrival(&next, source + Vec3::X).unwrap().is_finite());
                links += 1;
            }
            if e.get("classname") == Some("trigger_portal_cleanser") {
                let (lo, hi) = bounds[e.model().unwrap().unwrap()];
                let center = e.origin().unwrap() + (lo + hi) * 0.5;
                let mut entities = vec![e.clone()];
                entities.extend(
                    parse_entities(
                        format!(
                            "{{\"classname\" \"prop_physics\" \"origin\" \"{} {} {}\"}}",
                            center.x, center.y, center.z
                        )
                        .as_bytes(),
                    )
                    .unwrap(),
                );
                let mut r = Runtime::from_entities(entities, bounds.clone()).unwrap();
                r.portal_events = Some(vec![]);
                let cube = bounds.len();
                let mut w = World::from_brushes(vec![]);
                let mut p = Player::new(center - Vec3::Z * 32., 0.);
                r.tick(1. / 120., &mut p, &mut w);
                assert!(r.cleanse_portals, "{name}: {:?}", e.get("hammerid"));
                assert!(
                    !r.visible[cube],
                    "real cleanser must remove physics cube {name}"
                );
                cleansers += 1;
            }
        }
    }
    assert_eq!((maps.len(), links, cleansers), (26, 46, 40));
    println!("inventory verified: {} maps / {links} landmark links / {cleansers} actual cleanser brush volumes",maps.len());
}
#[test]
fn held_cube_cleanser_clears_handle_without_generic_game_leakage() {
    let es=parse_entities(br#"{"classname" "trigger_portal_cleanser" "model" "*1" "origin" "100 0 0" "OnDissolveBox" "marker,Kill,,0,-1"}{"classname" "logic_relay" "targetname" "marker"}{"classname" "prop_physics" "origin" "64 0 64"}"#).unwrap();
    let bounds = vec![
        (Vec3::ZERO, Vec3::ZERO),
        (Vec3::new(-2., -64., 0.), Vec3::new(2., 64., 128.)),
    ];
    let mut r = Runtime::from_entities(es, bounds).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    assert!(r.use_player(&p, &w));
    p.position.x = 40.;
    r.tick(1. / 120., &mut p, &mut w);
    assert!(
        r.visible[2],
        "generic Source game must not apply Portal rules"
    );
    assert!(!r.cleanse_portals);
    r.portal_events = Some(vec![]);
    r.tick(1. / 120., &mut p, &mut w);
    assert!(!r.visible[2]);
    assert!(!r.throw_held(&p));
    assert!(!r.is_enabled("marker"));
}
#[test]
fn touch_changelevel_uses_trigger_volume_without_player_spawnflag() {
    let es=parse_entities(br#"{"classname" "trigger_changelevel" "model" "*1" "map" "testchmb_a_01" "landmark" "here"}{"classname" "info_landmark" "targetname" "here" "origin" "5 6 7"}"#).unwrap();
    let mut r = Runtime::from_entities(
        es,
        vec![
            (Vec3::ZERO, Vec3::ZERO),
            (Vec3::splat(-10.), Vec3::splat(10.)),
        ],
    )
    .unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::new(100., 0., 0.), 0.);
    r.tick(0.01, &mut p, &mut w);
    assert!(r.change_level.is_none());
    p.position = Vec3::ZERO;
    r.tick(0.01, &mut p, &mut w);
    let req = r.change_level.take().unwrap();
    assert_eq!(req.source, Vec3::new(5., 6., 7.));
    assert!(req
        .arrival(&[], p.position)
        .unwrap_err()
        .contains("missing destination landmark"));
}
#[test]
fn unsafe_map_transition_rejects_before_asset_lookup() {
    let root = portal_bsp_viewer::session::GameRoot::new("../portal-original/portal", 1).unwrap();
    let req = portal_bsp_viewer::entity::ChangeLevel {
        map: "../escape".into(),
        landmark: "x".into(),
        source: Vec3::ZERO,
    };
    assert!(root
        .prepare(&req, &Player::new(Vec3::ZERO, 0.), false)
        .err()
        .unwrap()
        .contains("unsafe"));
}
#[test]
fn transition_auto_outputs_do_not_replay_newgame_or_lose_finite_mapspawn() {
    let es=parse_entities(br#"{"classname" "logic_auto" "OnMapSpawn" "spawn,Disable,,0,1" "OnNewGame" "new,Disable,,0,1" "OnMapTransition" "transition,Disable,,0,1"}{"classname" "logic_relay" "targetname" "spawn"}{"classname" "logic_relay" "targetname" "new"}{"classname" "logic_relay" "targetname" "transition"}"#).unwrap();
    let previous = Runtime::from_entities(vec![], vec![]).unwrap();
    let mut r = Runtime::from_entities(es, vec![]).unwrap();
    r.enter_transition(&previous, Vec3::ZERO);
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    r.tick(0.01, &mut p, &mut w);
    assert!(
        !r.is_enabled("spawn"),
        "finite OnMapSpawn must survive preparation"
    );
    assert!(
        r.is_enabled("new"),
        "OnNewGame must not run for a transition"
    );
    assert!(!r.is_enabled("transition"));
}
#[test]
fn real_cleanser_fizzles_player_portals_only_when_enabled() {
    let b = map("testchmb_a_00");
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut p = Player::new(w.spawn, 0.);
    r.input("trigger_fryer_a00", "Disable", "");
    r.tick(0.01, &mut p, &mut w);
    p.position = Vec3::new(-1264., -864., 144.);
    r.tick(0.01, &mut p, &mut w);
    assert!(!r.cleanse_portals);
    r.input("trigger_fryer_a00", "Enable", "");
    r.tick(0.01, &mut p, &mut w);
    r.tick(0.01, &mut p, &mut w);
    assert!(r.cleanse_portals);
}
