use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, RenderPosition, World},
    scene::Scene,
};
#[test]
fn walking_then_standing_near_lift_boundary_is_fixed_tick_deterministic() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut reference = None;
    for cadence in [30, 60, 144, 0] {
        let mut w = World::parse(&b).unwrap();
        let mut r = Runtime::new(&b).unwrap();
        w.bind_scene(&scene);
        r.bind_animations(&scene);
        w.update_models(&r.origins, &r.visible);
        let mut p = Player::new(Vec3::new(-1472., -864., 200.), std::f32::consts::FRAC_PI_2);
        for _ in 0..240 {
            p.step(&w, Vec3::ZERO, false, 1. / 120.);
        }
        let relative = p.position.z - r.origins[37].z;
        let mut render = RenderPosition::default();
        let mut acc = 0f64;
        let mut tick = 0;
        let mut frame = 0;
        let mut max_height_error = 0f32;
        while tick < 1800 {
            acc += if cadence == 0 {
                [1. / 25., 1. / 240., 1. / 48., 1. / 144.][frame % 4]
            } else {
                1. / cadence as f64
            };
            while acc >= 1. / 120. && tick < 1800 {
                render.begin_step(&p);
                let before_relative = p.position.z - r.origins[37].z;
                r.tick(1. / 120., &mut p, &mut w);
                p.set_crouch(&w, false);
                p.step(
                    &w,
                    if (120..180).contains(&tick) {
                        Vec3::X
                    } else {
                        Vec3::ZERO
                    },
                    false,
                    1. / 120.,
                );
                if tick > 300 {
                    assert!(
                        (p.position.z - r.origins[37].z - before_relative).abs() < 0.1,
                        "stationary boundary jitter at tick {tick}"
                    );
                }
                max_height_error =
                    max_height_error.max((p.position.z - r.origins[37].z - relative).abs());
                assert!(
                    p.grounded && !p.crouched,
                    "cadence={cadence} tick={tick} feet={:?}",
                    p.position
                );
                tick += 1;
                acc -= 1. / 120.;
            }
            let visual = render.sample(&p, (acc * 120.) as f32);
            assert!(visual.is_finite());
            frame += 1;
        }
        println!("cadence={cadence} frames={frame} max_relative_error={max_height_error} feet={:?} train={:?}",p.position,r.origins[37]);
        assert!(
            max_height_error < 4.,
            "boundary detail step exceeded real model lip: {max_height_error}"
        );
        assert!(
            r.origins[37].z > 780.,
            "boundary ride stalled: {:?}",
            r.origins[37]
        );
        let state = (p.position, p.velocity, r.origins[37]);
        if let Some(expected) = reference {
            assert_eq!(state, expected);
        } else {
            reference = Some(state);
        }
    }
}
