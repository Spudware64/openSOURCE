use opensource::play::*;
#[test]
fn actual_map_spawn_settles_on_solid_floor() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let world = World::parse(&b).unwrap();
    assert!(!world.brushes.is_empty());
    let mut p = Player::new(world.spawn, world.yaw);
    for _ in 0..240 {
        p.step(&world, glam::Vec3::ZERO, false, 1. / 120.);
    }
    assert!(p.grounded, "position {:?}", p.position);
    assert!((p.position.z - world.spawn.z).abs() < 100.);
    let z = p.position.z;
    p.step(&world, glam::Vec3::ZERO, true, 1. / 120.);
    assert!(p.position.z > z);
    for _ in 0..240 {
        p.step(&world, glam::Vec3::ZERO, false, 1. / 120.);
    }
    assert!(p.grounded);
    assert!(
        (p.position.z - z).abs() < 0.1,
        "jump must land on same floor"
    );
    let start = p.position;
    for _ in 0..30 {
        p.step(&world, glam::Vec3::X, false, 1. / 120.);
    }
    assert!(p.position.distance(start) > 5., "W must walk");
    for _ in 0..1200 {
        p.step(&world, glam::Vec3::X, false, 1. / 120.);
    }
    assert!(
        p.position.distance(start) < 500.,
        "must stop at actual chamber wall"
    );
    assert!(p.grounded);
    eprintln!(
        "spawn {:?}, settled {:?}, walked to {:?}",
        world.spawn, start, p.position
    );
}
#[test]
fn rejects_nonfinite_spawn_angle() {
    let mut b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let entities = opensource::lump(&b, 0, 1).unwrap();
    let off = entities.as_ptr() as usize - b.as_ptr() as usize;
    let text = std::str::from_utf8(entities).unwrap();
    let player = text.find("\"info_player_start\"").unwrap();
    let begin = text[..player].rfind('{').unwrap();
    let end = player + text[player..].find('}').unwrap();
    let angles = begin + text[begin..end].find("\"angles\"").unwrap();
    let start = off + angles + text[angles..end].find("180").unwrap();
    b[start..start + 3].copy_from_slice(b"NaN");
    assert!(World::parse(&b).is_err());
}
#[test]
fn rejects_unknown_collision_layout() {
    let mut b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    for id in [1, 18, 19] {
        let h = 8 + id * 16 + 8;
        let original = b[h..h + 4].to_vec();
        b[h..h + 4].copy_from_slice(&99i32.to_le_bytes());
        assert!(World::parse(&b).is_err(), "lump {id}");
        b[h..h + 4].copy_from_slice(&original);
    }
}
