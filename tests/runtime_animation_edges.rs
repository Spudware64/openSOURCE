use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};
#[test]
fn tracktrain_without_brush_model_is_rejected() {
    let entities=parse_entities(br#"{"classname" "path_track" "targetname" "p"} {"classname" "func_tracktrain" "target" "p"}"#).unwrap();
    assert!(Runtime::from_entities(entities, vec![(Vec3::ZERO, Vec3::ZERO)]).is_err());
}
#[test]
fn skeletal_completion_dispatches_authored_output_once() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = opensource::scene::Scene::load(&b, &fs).unwrap();
    let mut entities = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    entities
        .iter_mut()
        .find(|e| e.get("targetname") == Some("box_dropper_model_cover"))
        .unwrap()
        .pairs
        .push((
            "OnAnimationDone".into(),
            "elevator_door_model_middle,SetAnimation,open,0,1".into(),
        ));
    let bounds = opensource::lump(&b, 14, 48)
        .unwrap()
        .chunks_exact(48)
        .map(|a| {
            let f = |o| f32::from_le_bytes(a[o..o + 4].try_into().unwrap());
            (Vec3::new(f(0), f(4), f(8)), Vec3::new(f(12), f(16), f(20)))
        })
        .collect();
    let mut r = Runtime::from_entities(entities, bounds).unwrap();
    r.bind_animations(&scene);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::spawn(&w);
    r.input("box_dropper_model_cover", "SetAnimation", "open");
    for _ in 0..90 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert_eq!(r.animation_done_count("box_dropper_model_cover"), 1);
    let (sequence, time) = r.animation_pose("elevator_door_model_middle").unwrap();
    assert_eq!(sequence, "open");
    assert!(time > 0.);
    r.input("elevator_door_model_middle", "SetAnimation", "close");
    r.input("box_dropper_model_cover", "SetAnimation", "close");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert_eq!(r.animation_done_count("box_dropper_model_cover"), 2);
    assert_eq!(
        r.animation_pose("elevator_door_model_middle").unwrap().0,
        "close",
        "one-shot output must not retrigger"
    );
}
