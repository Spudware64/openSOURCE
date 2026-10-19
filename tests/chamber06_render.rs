use glam::Vec3;
use opensource::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};

#[test]
fn chamber06_rendered_panels_share_collision_parent_pose_at_spawn_and_during_motion() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_06.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.bind_animations(&scene);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::new(704., 128., 88.), 0.);
    p.noclip = true;
    for model in [22, 23, 27, 28, 32, 33] {
        let resolved = r.model_transforms().0[model];
        assert!(scene.origins[model].distance(resolved) < 0.001,
            "rendered panel {model} spawns extended at {:?} instead of retracted collision pose {resolved:?}", scene.origins[model]);
    }
    r.input("fling_pillar2_door_bottom_bottom_a2", "Close", "");
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    scene.animate(&r).unwrap();
    assert!(
        scene.origins[33].distance(r.model_transforms().0[33]) < 0.001,
        "brush child must follow its moving piston in the rendered scene"
    );
    // Props already bake the parent pose into their animated vertices: do not
    // apply it again in the model-origin buffer.
    let prop = r.entity_model("portalgun_spawn_rm5a").unwrap();
    assert_eq!(scene.origins[prop], r.origins[prop]);
}
