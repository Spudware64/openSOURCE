use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn door_leaves_use_authored_idle_pose_and_do_not_duplicate_on_open() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut runtime = Runtime::new(&b).unwrap();
    let mut world = World::parse(&b).unwrap();
    let mut player = Player::new(Vec3::new(-620., -864., 144.1), 180.);
    let count = scene.vertices.len();
    for id in [16, 17] {
        let leaf = scene
            .vertices
            .iter()
            .filter(|v| v.model == id)
            .collect::<Vec<_>>();
        assert!(leaf.len() > 300);
        let (min, max) = leaf.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(a, b), v| {
                let p = Vec3::from(v.position) + runtime.origins[id as usize];
                (a.min(p), b.max(p))
            },
        );
        assert!(
            max.x - min.x < 16.,
            "door must face X, not be edge-on: {min:?} {max:?}"
        );
        assert!(
            max.y - min.y > 55.,
            "each authored leaf covers half the opening"
        );
        assert!(min.y >= -922. && max.y <= -806.);
        assert!((min.z - 128.).abs() < 0.1);
    }
    runtime.input("exit_door_right_a00", "Open", "");
    runtime.input("exit_door_left_a00", "Open", "");
    for _ in 0..120 {
        runtime.tick(1. / 120., &mut player, &mut world);
    }
    assert_eq!(scene.vertices.len(), count);
    assert!((runtime.origins[16].y - scene.origins[16].y - 44.).abs() < 0.1);
    assert!((runtime.origins[17].y - scene.origins[17].y + 44.).abs() < 0.1);
}
