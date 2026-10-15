use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    scene::Scene,
};
#[test]
fn authored_spawn_and_elevator_height_trace() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    for elevator in [false, true] {
        let mut w = World::parse(&b).unwrap();
        let mut r = Runtime::new(&b).unwrap();
        w.bind_scene(&scene);
        r.bind_animations(&scene);
        w.update_models(&r.origins, &r.visible);
        let mut p = if elevator {
            Player::new(Vec3::new(-1472., -864., 200.), 0.)
        } else {
            Player::spawn(&w)
        };
        let mut previous = 0.;
        let mut max_delta = 0f32;
        for tick in 0..2400 {
            r.tick(1. / 120., &mut p, &mut w);
            p.set_crouch(&w, false);
            p.step(&w, Vec3::ZERO, false, 1. / 120.);
            let relative = p.position.z - if elevator { r.origins[37].z } else { 0. };
            if tick > 120 {
                max_delta = max_delta.max((relative - previous).abs());
            }
            previous = relative;
            if tick % 120 == 0 {
                println!("height elevator={elevator} tick={tick} feet={:?} eye={} grounded={} relative={relative}",p.position,p.eye_height(),p.grounded);
            }
            assert!(!p.crouched);
        }
        assert!(
            max_delta < 0.1,
            "elevator={elevator} maximum relative height jump {max_delta}"
        );
    }
}
