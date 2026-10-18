use opensource::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn map_setanimation_changes_scene_and_fires_done_once() {
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
        .find(|a| a.name == "box_dropper_model_cover")
        .unwrap();
    let range = instance.range.clone();
    let before = scene.vertices[range.clone()].to_vec();
    runtime.input("box_dropper_model_cover", "SetAnimation", "open");
    for _ in 0..90 {
        runtime.tick(1. / 120., &mut player, &mut world);
    }
    scene.animate(&runtime).unwrap();
    assert!(scene.vertices[range]
        .iter()
        .zip(before)
        .any(|(a, b)| glam::Vec3::from(a.position).distance(glam::Vec3::from(b.position)) > 1.));
    assert!(!runtime.unknown_inputs.contains("prop_dynamic:SetAnimation"));
    assert_eq!(runtime.animation_done_count("box_dropper_model_cover"), 1);
    runtime.input("box_dropper_model_cover", "SetAnimation", "bad_sequence");
    runtime.tick(1. / 120., &mut player, &mut world);
    assert!(runtime
        .unknown_inputs
        .iter()
        .any(|s| s.contains("bad_sequence")));
}
