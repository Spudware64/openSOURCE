use glam::Vec3;
use portal_bsp_viewer::play::{Plane, Player, World};

#[test]
fn grounded_input_accelerates_and_release_slows_without_instant_stop() {
    let world = World::from_brushes(vec![vec![
        Plane {
            normal: Vec3::Z,
            distance: 0.,
        },
        Plane {
            normal: -Vec3::Z,
            distance: 100.,
        },
        Plane {
            normal: Vec3::X,
            distance: 10000.,
        },
        Plane {
            normal: -Vec3::X,
            distance: 10000.,
        },
        Plane {
            normal: Vec3::Y,
            distance: 10000.,
        },
        Plane {
            normal: -Vec3::Y,
            distance: 10000.,
        },
    ]]);
    let mut player = Player::new(Vec3::new(0., 0., 1.), 0.);
    player.step(&world, Vec3::X, false, 1. / 120.);
    let first = player.velocity.x;
    assert!(first > 0. && first < 30., "first-tick speed {first}");
    for _ in 0..60 {
        player.step(&world, Vec3::X, false, 1. / 120.);
    }
    let running = player.velocity.x;
    assert!((running - 190.).abs() < 0.1, "running speed {running}");
    player.step(&world, Vec3::ZERO, false, 1. / 120.);
    assert!(
        player.velocity.x > 0. && player.velocity.x < running,
        "release speed {}",
        player.velocity.x
    );
    for _ in 0..120 {
        player.step(&world, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(player.velocity.x.abs() < 0.1);
}
