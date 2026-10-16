use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn ordinary_corridor_entry_stays_on_visible_cabin_floor() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.bind_animations(&s);
    let mut w = World::parse(&b).unwrap();
    w.bind_scene(&s);
    w.update_models(&r.origins, &r.visible);
    // Stage outside the mouth, never on the cabin floor; all boundary crossing is ordinary input.
    let mut p = Player::new(Vec3::new(-1360., -864., 144.1), 0.);
    for _ in 0..40 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    let a = s
        .animations
        .iter()
        .find(|a| a.name == "elevator_body_model")
        .unwrap();
    let floor = s.vertices[a.range.clone()]
        .chunks_exact(3)
        .filter(|t| Vec3::from(t[0].normal).z > 0.99)
        .filter_map(|t| {
            let ps = [
                Vec3::from(t[0].position),
                Vec3::from(t[1].position),
                Vec3::from(t[2].position),
            ];
            let n = (ps[1] - ps[0]).cross(ps[2] - ps[0]).normalize_or_zero();
            if n.z.abs() < 0.9999 {
                return None;
            }
            let xy = Vec3::new(0., 0., 0.);
            let signs = (0..3)
                .map(|i| (ps[(i + 1) % 3] - ps[i]).cross(xy - ps[i]).z)
                .collect::<Vec<_>>();
            if !(signs.iter().all(|x| *x >= -0.01) || signs.iter().all(|x| *x <= 0.01)) {
                return None;
            }
            let z = n.dot(ps[0]) / n.z + r.origins[37].z;
            (z > 140. && z < 150.).then_some(z)
        })
        .next()
        .expect("authored floor triangles");
    for tick in 0..110 {
        let walk = p.position.x > -1470.;
        r.tick(1. / 120., &mut p, &mut w);
        p.step(
            &w,
            if walk { -Vec3::X } else { Vec3::ZERO },
            false,
            1. / 120.,
        );
        assert!(
            p.position.x > -1420. || p.position.z > floor - 0.2,
            "entry tick {tick}: feet {:?}, rendered floor {floor}",
            p.position
        );
    }
    assert!(
        p.position.x < -1460.,
        "must actually enter {:?}",
        p.position
    );
    assert!(p.grounded);
    assert!(
        (p.position.z - floor).abs() < 0.2,
        "{:?} floor {floor}",
        p.position
    );
}
