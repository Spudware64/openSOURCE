use glam::Vec3;
use opensource::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn actual_pedestal_child_mesh_follows_rotating_parent() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut s = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.bind_animations(&s);
    let a = s
        .animations
        .iter()
        .find(|a| a.name == "pedistal_center_lvl2")
        .unwrap();
    let range = a.range.clone();
    let before = Vec3::from(s.vertices[range.start].position);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&s);
    let mut p = Player::new(w.spawn, w.yaw);
    p.step(&w, Vec3::ZERO, false, 1. / 120.);
    r.input("tca01_rotate_launcher_door1", "Open", "");
    for _ in 0..180 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    s.animate(&r).unwrap();
    assert!(
        before.distance(Vec3::from(s.vertices[range.start].position)) > 1.,
        "child mesh remained frozen"
    );
    assert!(
        s.animations.iter().any(|a| a.name == "portalgun"),
        "actual weapon script world model absent"
    );
}
