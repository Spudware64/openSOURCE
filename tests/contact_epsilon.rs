use glam::Vec3;
use opensource::play::{Plane, Player, World};
fn floor() -> World {
    World::from_brushes(vec![vec![
        Plane {
            normal: Vec3::Z,
            distance: 0.,
        },
        Plane {
            normal: -Vec3::Z,
            distance: 16.,
        },
        Plane {
            normal: Vec3::X,
            distance: 1000.,
        },
        Plane {
            normal: -Vec3::X,
            distance: 1000.,
        },
        Plane {
            normal: Vec3::Y,
            distance: 1000.,
        },
        Plane {
            normal: -Vec3::Y,
            distance: 1000.,
        },
    ]])
}
#[test]
fn sub_epsilon_floor_clearance_does_not_drop_player_through_ground() {
    for dt in [1. / 60., 1. / 120., 1. / 240.] {
        for clearance in [0.0001, 0.015625, 0.03124, 0.03125, 0.03126] {
            let w = floor();
            let mut p = Player::new(Vec3::new(0., 0., clearance), 0.);
            for tick in 0..240 {
                p.step(&w, if tick < 120 { Vec3::X } else { Vec3::ZERO }, false, dt);
                assert!(
                    p.position.z >= 0. && p.position.z <= 0.032,
                    "dt={dt} clearance={clearance} tick={tick} feet={:?}",
                    p.position
                );
                assert!(p.grounded);
            }
        }
    }
}
