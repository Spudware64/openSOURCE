use glam::Vec3;
use opensource::{assets::Assets, entity::*, play::*, scene::Scene};
#[test]
fn actual_second_room_cube_settles_on_plate_and_keeps_exit_open() {
    run(false);
}
#[test]
fn actual_second_room_player_to_cube_handoff_keeps_exit_open() {
    run(true);
}
fn run(player_first: bool) {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let es = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.bind_animations(&scene);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&scene);
    let cube = opensource::lump(&b, 14, 48).unwrap().len() / 48
        + es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .position(|e| e.get("targetname") == Some("box"))
            .unwrap();
    let door = es
        .iter()
        .find(|e| e.get("targetname") == Some("door_right_00"))
        .unwrap()
        .model()
        .unwrap()
        .unwrap();
    let closed = r.origins[door];

    // Explicitly staged free drop above the real second-room plate, not an input-only solve.
    r.origins[cube] = Vec3::new(-1376., -572., 650.);
    let mut p = Player::new(Vec3::new(-700., -950., 140.1), 0.);
    assert!(
        !w.overlaps(p.position, p.height()),
        "staged observer must be outside solids"
    );
    let bottom = scene
        .vertices
        .iter()
        .filter(|v| v.model as usize == cube)
        .map(|v| v.position[2])
        .fold(f32::INFINITY, f32::min);
    if player_first {
        p.position = Vec3::new(-1338., -572., 600.);
        for _ in 0..120 {
            r.tick(1. / 120., &mut p, &mut w);
        }
        p.position = Vec3::new(-700., -950., 140.1);
    }
    for _ in 0..360 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    let settled = r.origins[cube];
    let plate_top = scene
        .animations
        .iter()
        .find(|a| a.name == "button_top_a_rm1a")
        .unwrap();
    let plate_model = scene.vertices[plate_top.range.start].model as usize;
    let local_top = scene.vertices[plate_top.range.clone()]
        .iter()
        .map(|v| v.position[2])
        .fold(f32::NEG_INFINITY, f32::max);
    eprintln!(
        "cube {cube} origin {settled:?} bottom {} door {:?} closed {closed:?}",
        settled.z + bottom,
        r.origins[door]
    );
    assert!(
        r.origins[door].distance(closed) > 10.,
        "cube must keep authored door open"
    );
    assert!(
        (settled.z + bottom - (r.origins[plate_model].z + local_top)).abs() < 0.1,
        "cube must rest on posed plate rather than BSP floor"
    );
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[cube].distance(settled) < 0.1);
    }
    // Player overlaps the plate beside the cube, then leaves: cube ownership persists.
    p.position = Vec3::new(-1338., -572., r.origins[plate_model].z + local_top + 0.1);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    p.position = Vec3::new(-700., -950., 140.1);
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[door].distance(closed) > 10.,
        "cube/player handoff must not release door"
    );
    let hi = scene
        .vertices
        .iter()
        .filter(|v| v.model as usize == cube)
        .map(|v| Vec3::from(v.position))
        .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
    let lo = scene
        .vertices
        .iter()
        .filter(|v| v.model as usize == cube)
        .map(|v| Vec3::from(v.position))
        .fold(Vec3::splat(f32::INFINITY), Vec3::min);
    assert!(
        r.rapier_collider_center(cube)
            .unwrap()
            .distance(r.origins[cube] + (hi + lo) * 0.5)
            < 0.01
    );
    // The normal E-use path repicks the resting cube; no forced button/door I/O.
    p.position = Vec3::new(-1306., -572., 592.1);
    p.yaw = std::f32::consts::PI;
    p.pitch =
        (r.rapier_collider_center(cube).unwrap().z - p.position.z - p.eye_height()).atan2(70.);
    assert!(r.use_player(&p, &w), "resting cube must be pickable");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[door].distance(closed) < 0.1,
        "repicking must release the plate and close the exit"
    );
}
