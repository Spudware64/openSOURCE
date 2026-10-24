use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};
#[test]
fn physical_prop_falls_and_stays_finite() {
    let entities=parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "model" "models/props/metal_box.mdl" "origin" "0 0 100"}"#).unwrap();
    let mut r = Runtime::from_entities(entities, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins.len() > 1,
        "physics models need a runtime transform slot"
    );
    assert!(
        r.origins[1].z < 0.,
        "gravity must advance the actual runtime prop"
    );
    assert!(r.origins[1].is_finite());
}
#[test]
fn maker_spawns_real_template_instances_with_bounded_slots() {
    let es=parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "model" "models/props/metal_box.mdl" "origin" "0 0 100"}{"classname" "point_template" "targetname" "template" "Template01" "box" "spawnflags" "3"}{"classname" "env_entity_maker" "targetname" "maker" "EntityTemplate" "template" "origin" "50 0 200"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    r.input("maker", "ForceSpawn", "");
    r.tick(1. / 120., &mut p, &mut w);
    assert_eq!(r.visible.iter().filter(|v| **v).count(), 3);
    assert!(r
        .origins
        .iter()
        .any(|v| v.distance(Vec3::new(50., 0., 200.)) < 1.));
    for _ in 0..20 {
        r.input("maker", "ForceSpawn", "");
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.origins.len() <= 10);
}
#[test]
fn real_box_weights_chamber_button_without_player_on_plate() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::new(-650., -1000., 136.), 0.);
    let es = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    let m = opensource::lump(&b, 14, 48).unwrap().len() / 48
        + es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .position(|e| e.get("targetname") == Some("box"))
            .unwrap();
    r.origins[m] = Vec3::new(-511.87, -928.49, 170.);
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.origins[16].y> -800.,"cube must open authored exit-door output: box {:?}, doors {:?}, unknown {:?}, player overlap {}",r.origins[m],r.origins[16],r.unknown_inputs,w.overlaps(p.position,p.height()));
}
#[test]
fn free_cube_crosses_linked_portal_with_velocity() {
    use opensource::portal::Portal;
    let es =
        parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "origin" "10 0 64"}"#)
            .unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::new(-54., 0., 0.), 0.);
    assert!(r.use_player(&p, &w));
    p.yaw = std::f32::consts::PI;
    assert!(r.throw_held(&p));
    w.portals = [
        Some(Portal::new(Vec3::new(0., 0., 64.), Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(100., 0., 64.), Vec3::X, Vec3::Z)),
    ];
    for _ in 0..6 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[1].x > 100.,
        "mapped cube position {:?}",
        r.origins[1]
    );
}
#[test]
fn real_spawn_pickup_walk_drop_opens_door() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = opensource::scene::Scene::load(&b, &fs).unwrap();
    r.bind_animations(&scene);
    w.bind_scene(&scene);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(Vec3::new(-740., -1056., 128.1), std::f32::consts::PI);
    p.pitch = -0.45;
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(r.use_player(&p, &w), "pickup at {:?}", p.position);
    let target = Vec3::new(-572., -928., p.position.z);
    for _ in 0..360 {
        let delta = target - p.position;
        p.yaw = delta.y.atan2(delta.x);
        r.tick(1. / 120., &mut p, &mut w);
        p.step(
            &w,
            if delta.truncate().length() > 2. {
                Vec3::X
            } else {
                Vec3::ZERO
            },
            false,
            1. / 120.,
        );
    }
    assert!(
        p.position.truncate().distance(target.truncate()) < 3.,
        "walk {:?}",
        p.position
    );
    p.yaw = 0.;
    p.pitch = -0.6;
    for _ in 0..5 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.use_player(&p, &w));
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(
        r.origins[16].y > -800.,
        "door {:?}, player {:?}, objects {:?}",
        r.origins[16],
        p.position,
        &r.origins[92..100]
    );
    for target in [
        Vec3::new(-970., -864., p.position.z),
        Vec3::new(-572., -928., p.position.z),
    ] {
        for _ in 0..360 {
            let delta = target - p.position;
            p.yaw = delta.y.atan2(delta.x);
            p.pitch = 0.;
            r.tick(1. / 120., &mut p, &mut w);
            p.step(
                &w,
                if delta.truncate().length() > 2. {
                    Vec3::X
                } else {
                    Vec3::ZERO
                },
                false,
                1. / 120.,
            );
        }
        assert!(
            p.position.truncate().distance(target.truncate()) < 3.,
            "must walk through weighted-open door and back without noclip: {:?}",
            p.position
        );
    }
    p.yaw = 0.;
    p.pitch = -0.6;
    assert!(r.use_player(&p, &w), "pick cube back up off plate");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        (r.origins[16].y + 836.).abs() < 0.01,
        "cube pickup must release pressure and close door"
    );
    let before = r.origins.clone();
    for dt in [f32::NAN, f32::INFINITY, -1., 0., 0.11] {
        r.tick(dt, &mut p, &mut w);
    }
    assert_eq!(before, r.origins, "invalid physics timesteps must be no-op");
}
#[test]
fn throw_key_is_distinct_from_portal_mouse_buttons() {
    use opensource::play::{Action, Controls};
    assert_eq!(
        Controls::default().press(winit::keyboard::KeyCode::KeyF),
        Action::Throw
    );
}
fn maker_in_floor(flags: u32) -> Runtime {
    let fixture = r#"
{"classname" "prop_physics" "targetname" "box" "origin" "0 0 100"}
{"classname" "point_template" "targetname" "template" "Template01" "box"}
{"classname" "env_entity_maker" "targetname" "maker" "EntityTemplate" "template"
 "origin" "0 0 -100" "spawnflags" "$FLAGS"
 "OnEntitySpawned" "result,SetActivatedState,1,0,-1"
 "OnEntityFailedSpawn" "result,SetActivatedState,0,0,-1"}
{"classname" "prop_portal" "targetname" "result"}
"#.replace("$FLAGS", &flags.to_string());
    let es = parse_entities(fixture.as_bytes()).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::from_brushes(vec![vec![opensource::play::Plane {
        normal: Vec3::Z,
        distance: 0.,
    }]]);
    let mut p = Player::new(Vec3::new(200., 0., 100.), 0.);
    r.input("maker", "ForceSpawn", "");
    r.tick(1. / 120., &mut p, &mut w);
    r
}
#[test]
fn blocked_maker_does_not_embed_or_fire_spawn_output() {
    let r = maker_in_floor(8);
    assert_eq!(r.visible.iter().filter(|v| **v).count(), 2);
    assert_eq!(r.portal_events.unwrap(), vec![("result".into(), false)]);
}
#[test]
fn forced_maker_without_space_check_spawns_and_fires_success_in_solid() {
    let r = maker_in_floor(0);
    assert_eq!(r.visible.iter().filter(|v| **v).count(), 3);
    assert_eq!(r.portal_events.unwrap(), vec![("result".into(), true)]);
}
#[test]
fn physics_trigger_delivers_specific_activator() {
    let es=parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "origin" "0 0 100"}{"classname" "trigger_multiple" "model" "*1" "spawnflags" "8" "origin" "0 0 100" "OnStartTouch" "!activator,Kill,,0,-1"}"#).unwrap();
    let mut r = Runtime::from_entities(
        es,
        vec![
            (Vec3::ZERO, Vec3::ZERO),
            (Vec3::splat(-30.), Vec3::splat(30.)),
        ],
    )
    .unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::new(500., 0., 100.), 0.);
    r.tick(1. / 120., &mut p, &mut w);
    assert!(!r.visible[2]);
}
#[test]
fn collision_extent_comes_from_real_posed_mesh() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = opensource::scene::Scene::load(&b, &fs).unwrap();
    r.bind_animations(&scene);
    let es = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    let m = opensource::lump(&b, 14, 48).unwrap().len() / 48
        + es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .position(|e| e.get("targetname") == Some("box"))
            .unwrap();
    let low = scene
        .vertices
        .iter()
        .filter(|v| v.model as usize == m)
        .map(|v| v.position[2])
        .fold(f32::INFINITY, f32::min);
    let mut w = World::from_brushes(vec![vec![opensource::play::Plane {
        normal: Vec3::Z,
        distance: 0.,
    }]]);
    let mut p = Player::new(Vec3::new(500., 0., 100.), 0.);
    r.origins[m] = Vec3::new(0., 0., 100.);
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        (r.origins[m].z + low - 0.03125).abs() < 0.01,
        "mesh bottom must match collision floor: {:?}, bound {low}",
        r.origins[m]
    );
}
#[test]
fn bounded_cuboids_stack_on_each_other() {
    let es=parse_entities(br#"{"classname" "prop_physics" "targetname" "lower" "origin" "0 0 100"}{"classname" "prop_physics" "targetname" "upper" "origin" "0 0 160"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![vec![opensource::play::Plane {
        normal: Vec3::Z,
        distance: 0.,
    }]]);
    let mut p = Player::new(Vec3::new(500., 0., 100.), 0.);
    for _ in 0..600 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!((r.origins[1].z - 20.03125).abs() < 0.1);
    assert!(
        r.origins[2].z - r.origins[1].z > 39.9,
        "stack {:?}",
        &r.origins[..3]
    );
}
#[test]
fn player_cannot_walk_through_resting_cube() {
    let es =
        parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "origin" "64 0 20.04"}"#)
            .unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![vec![opensource::play::Plane {
        normal: Vec3::Z,
        distance: 0.,
    }]]);
    let mut p = Player::new(Vec3::new(0., 0., 0.04), 0.);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert!(p.position.x < 29., "walked through cube {:?}", p.position);
}
#[test]
fn cube_portal_exit_is_blocked_by_other_body() {
    use opensource::portal::Portal;
    let es=parse_entities(br#"{"classname" "prop_physics" "targetname" "source" "origin" "10 0 64"}{"classname" "prop_physics" "targetname" "blocker" "origin" "100 0 64"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::new(-54., 0., 0.), 0.);
    assert!(r.use_player(&p, &w));
    p.yaw = std::f32::consts::PI;
    assert!(r.throw_held(&p));
    w.portals = [
        Some(Portal::new(Vec3::new(0., 0., 64.), Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(100., 0., 64.), Vec3::X, Vec3::Z)),
    ];
    for _ in 0..6 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[1].x < 50.,
        "teleported into occupied exit {:?}",
        r.origins[1]
    );
}
#[test]
fn actual_chamber_cube_falls_through_floor_portal_and_exits_wall() {
    use opensource::{assets::Assets, portal::Surfaces};
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let surfaces = Surfaces::load(&b, &fs).unwrap();
    let floor = surfaces
        .place(Vec3::new(-416., -320., 178.), -Vec3::Z)
        .unwrap();
    let wall = surfaces
        .place(Vec3::new(-1088., -600., 640.), -Vec3::Y)
        .unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    w.portals = [Some(floor), Some(wall)];
    let mut p = Player::new(w.spawn, w.yaw);
    let es = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    let m = opensource::lump(&b, 14, 48).unwrap().len() / 48
        + es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .position(|e| e.get("targetname") == Some("box"))
            .unwrap();
    r.origins[m] = floor.center + Vec3::Z * 80.;
    let mut crossed = false;
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        if r.origins[m].distance(floor.center) > 300. {
            crossed = true;
            break;
        }
    }
    assert!(
        crossed,
        "actual cube never cleared floor backing {:?}",
        r.origins[m]
    );
    let before = r.origins[m];
    r.tick(1. / 120., &mut p, &mut w);
    assert!(
        (r.origins[m] - before).dot(wall.normal) > 2.,
        "fall momentum must become wall-exit horizontal speed {:?} -> {:?}",
        before,
        r.origins[m]
    );
}
#[test]
fn physics_population_and_malformed_transforms_are_bounded() {
    let es = parse_entities(br#"{"classname" "prop_physics" "origin" "NaN 0 0"}"#).unwrap();
    assert!(Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).is_err());
    let one = parse_entities(br#"{"classname" "prop_physics" "origin" "0 0 100"}"#).unwrap();
    assert!(
        Runtime::from_entities(vec![one[0].clone(); 129], vec![(Vec3::ZERO, Vec3::ZERO)]).is_err(),
        "untrusted BSP may not create unbounded quadratic collision work"
    );
}
#[test]
fn use_picks_up_drops_and_throw_retains_momentum() {
    let es=parse_entities(br#"{"classname" "prop_physics" "targetname" "box" "model" "models/props/metal_box.mdl" "origin" "64 0 64"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    assert!(r.use_player(&p, &w));
    p.position.x = 10.;
    r.tick(1. / 120., &mut p, &mut w);
    assert!((r.origins[1].x - 74.).abs() < 1.);
    assert!(r.use_player(&p, &w));
    r.tick(1. / 120., &mut p, &mut w);
    assert!(r.origins[1].z < 64.);
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    r.tick(1. / 120., &mut p, &mut w);
    assert!(r.origins[1].x > 74.);
}
