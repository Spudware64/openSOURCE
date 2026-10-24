use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};

#[test]
fn look_trigger_emits_touch_edges_independently_of_gaze() {
    let entities = parse_entities(
        br#"
{"classname" "trigger_look" "model" "*0" "target" "aim" "spawnflags" "1" "LookTime" "0.2"
 "OnStartTouch" "enter,SetActivatedState,1,0,-1"
 "OnStartTouchAll" "enter_all,SetActivatedState,1,0,-1"
 "OnEndTouch" "leave,SetActivatedState,1,0,-1"
 "OnEndTouchAll" "leave_all,SetActivatedState,1,0,-1"
 "OnTrigger" "gaze,SetActivatedState,1,0,-1"}
{"classname" "info_target" "targetname" "aim" "origin" "1000 0 64"}
{"classname" "prop_portal" "targetname" "enter"}
{"classname" "prop_portal" "targetname" "enter_all"}
{"classname" "prop_portal" "targetname" "leave"}
{"classname" "prop_portal" "targetname" "leave_all"}
{"classname" "prop_portal" "targetname" "gaze"}
"#,
    )
    .unwrap();
    let mut runtime =
        Runtime::from_entities(entities, vec![(Vec3::splat(-64.), Vec3::splat(64.))]).unwrap();
    runtime.portal_events = Some(vec![]);
    let mut world = World::from_brushes(vec![]);
    let mut player = Player::new(Vec3::ZERO, std::f32::consts::PI);
    player.position.x = 200.;
    runtime.tick(0.1, &mut player, &mut world);
    assert!(runtime.portal_events.as_ref().unwrap().is_empty());
    player.position = Vec3::ZERO;
    for _ in 0..10 {
        runtime.tick(0.1, &mut player, &mut world);
    }
    let events = |names: &[&str]| {
        names
            .iter()
            .map(|name| ((*name).into(), true))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        runtime.portal_events.as_ref().unwrap(),
        &events(&["enter", "enter_all"])
    );
    player.position.x = 200.;
    for _ in 0..3 {
        runtime.tick(0.1, &mut player, &mut world);
    }
    assert_eq!(
        runtime.portal_events.as_ref().unwrap(),
        &events(&["enter", "enter_all", "leave", "leave_all"])
    );
    player.position = Vec3::ZERO;
    player.yaw = 0.;
    for _ in 0..3 {
        runtime.tick(0.1, &mut player, &mut world);
    }
    assert_eq!(
        runtime.portal_events.as_ref().unwrap(),
        &events(&[
            "enter",
            "enter_all",
            "leave",
            "leave_all",
            "enter",
            "enter_all",
            "gaze"
        ])
    );
}
