use glam::Vec3;
use opensource::{entity::*, play::*};
#[test]
fn actual_player_pressure_trigger_opens_and_releases_real_doors() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut p = Player::new(Vec3::new(-511.87, -928.49, 144.1), 0.);
    let original = r.origins[16];
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[16].distance(original) > 10.,
        "{:?}",
        r.origins[16]
    );
    p.position = Vec3::new(-700., -950., 140.1);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.origins[16].distance(original) < 0.1);
}
#[test]
fn use_button_runs_delayed_relay_output_once() {
    let e=parse_entities(br#"{"classname" "func_button" "model" "*1" "targetname" "button" "origin" "40 0 64" "OnPressed" "relay,Trigger,,0.1,1"}{"classname" "logic_relay" "targetname" "relay" "OnTrigger" "door,Open,,0,1"}{"classname" "func_door" "targetname" "door" "model" "*2" "origin" "200 0 0" "movedir" "0 90 0" "speed" "100" "wait" "-1"}"#).unwrap();
    let bounds = vec![
        (Vec3::ZERO, Vec3::ZERO),
        (Vec3::splat(-4.), Vec3::splat(4.)),
        (Vec3::ZERO, Vec3::new(10., 40., 72.)),
    ];
    let mut r = Runtime::from_entities(e, bounds).unwrap();
    let mut w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    assert!(r.use_player(&p, &w));
    for _ in 0..10 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert_eq!(r.origins[2].y, 0.);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.origins[2].y > 39.);
    assert!(r.unknown_inputs.is_empty(), "{:?}", r.unknown_inputs);
    r.input("door", "Close", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    r.input("button", "Press", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[2].y.abs() < 0.1,
        "one-shot output must not repeat"
    );
}
#[test]
fn real_sliding_door_stops_before_crushing_player() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    r.input("exit_door_right_a00", "Open", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    p.position = Vec3::new(-912., -836., 128.1);
    assert!(!w.overlaps(p.position, p.height()));
    r.input("exit_door_right_a00", "Close", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(!w.overlaps(p.position, p.height()));
    assert!(r.origins[16].y > -826.);
}
#[test]
fn close_use_target_is_not_rejected_by_own_collision_epsilon() {
    let e = parse_entities(
        br#"{"classname" "func_button" "model" "*1" "targetname" "button" "origin" "10 0 64"}"#,
    )
    .unwrap();
    let bounds = vec![
        (Vec3::ZERO, Vec3::ZERO),
        (Vec3::splat(-1.), Vec3::splat(1.)),
    ];
    let mut r = Runtime::from_entities(e, bounds).unwrap();
    let planes = vec![
        Plane {
            normal: Vec3::X,
            distance: 11.,
        },
        Plane {
            normal: -Vec3::X,
            distance: -9.,
        },
        Plane {
            normal: Vec3::Y,
            distance: 1.,
        },
        Plane {
            normal: -Vec3::Y,
            distance: 1.,
        },
        Plane {
            normal: Vec3::Z,
            distance: 65.,
        },
        Plane {
            normal: -Vec3::Z,
            distance: -63.,
        },
    ];
    let w = World::from_brushes(vec![planes]);
    let p = Player::new(Vec3::ZERO, 0.);
    assert!(r.use_player(&p, &w));
}
