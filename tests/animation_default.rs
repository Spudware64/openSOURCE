use opensource::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn authored_elevator_default_animation_and_completion_output() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut scene = Scene::load(&b, &fs).unwrap();
    let mut runtime = Runtime::new(&b).unwrap();
    runtime.bind_animations(&scene);
    let mut world = World::parse(&b).unwrap();
    let mut player = Player::spawn(&world);
    let instance = scene
        .animations
        .iter()
        .find(|a| a.name == "elevator_door_model_middle")
        .unwrap();
    let range = instance.range.clone();
    let before = scene.vertices[range.clone()].to_vec();
    runtime.input("elevator_door_model_open", "Trigger", "");
    for _ in 0..100 {
        runtime.tick(1. / 120., &mut player, &mut world);
    }
    scene.animate(&runtime).unwrap();
    assert!(scene.vertices[range.clone()]
        .iter()
        .zip(&before)
        .any(|(a, b)| glam::Vec3::from(a.position).distance(glam::Vec3::from(b.position)) > 10.));
    assert_eq!(
        runtime.animation_done_count("elevator_door_model_middle"),
        1
    );
    runtime.input("elevator_door_model_close", "Trigger", "");
    for _ in 0..200 {
        runtime.tick(1. / 120., &mut player, &mut world);
    }
    assert_eq!(
        runtime
            .animation_pose("elevator_door_model_middle")
            .unwrap()
            .0,
        "idle_closed"
    );
    assert_eq!(
        runtime.animation_done_count("elevator_door_model_middle"),
        2
    );
    scene.animate(&runtime).unwrap();
    assert!(scene.vertices[range]
        .iter()
        .zip(before)
        .all(|(a, b)| glam::Vec3::from(a.position).distance(glam::Vec3::from(b.position)) < 0.1));
    assert_eq!(
        scene.animate(&runtime).unwrap(),
        0,
        "stable pose must not reskin/reupload"
    );
    assert!(!runtime
        .unknown_inputs
        .iter()
        .any(|s| s.contains("SetDefaultAnimation")));
}
