use glam::Vec3;
use opensource::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};

#[test]
fn middle_lift_support_matches_actual_floor_not_expanded_wall_details() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    w.bind_scene(&scene);
    r.bind_animations(&scene);
    w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(Vec3::new(-1472., -864., 200.), 0.);
    for _ in 0..240 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    // The real posed mesh's broad floor is at Z=146.55, not the distant
    // decorative horizontal wall triangles at Z=178/189/193.
    assert!(
        (146.4..146.8).contains(&p.position.z),
        "false floor: {:?}",
        p.position
    );
    let relative = p.position.z - r.origins[37].z;
    for tick in 0..1800 {
        // Repeated input-only walks across floor-triangle and support boundaries.
        let phase = tick % 240;
        p.yaw = if phase < 120 {
            0.
        } else {
            std::f32::consts::PI
        };
        r.tick(1. / 120., &mut p, &mut w);
        p.step(
            &w,
            if phase < 30 || (120..150).contains(&phase) {
                Vec3::X
            } else {
                Vec3::ZERO
            },
            false,
            1. / 120.,
        );
        let z = p.position.z - r.origins[37].z;
        assert!(
            (z - relative).abs() < 0.1,
            "tick {tick}: support-relative height {z} expected {relative}; feet {:?}",
            p.position
        );
        assert!(p.grounded && !p.crouched);
        assert_eq!(p.eye_height(), 64.);
    }
    assert!(
        r.origins[37].z > 780.,
        "lift must complete, not stall against phantom support: {:?}",
        r.origins[37]
    );
}
