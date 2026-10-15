use glam::Vec3;
use portal_bsp_viewer::play::{Plane, World};
fn slab(top: f32) -> Vec<Plane> {
    vec![
        Plane {
            normal: Vec3::Z,
            distance: top,
        },
        Plane {
            normal: -Vec3::Z,
            distance: 16. - top,
        },
        Plane {
            normal: Vec3::X,
            distance: 64.,
        },
        Plane {
            normal: -Vec3::X,
            distance: 64.,
        },
        Plane {
            normal: Vec3::Y,
            distance: 64.,
        },
        Plane {
            normal: -Vec3::Y,
            distance: 64.,
        },
    ]
}
fn world(static_top: f32, moving_top: f32) -> World {
    let mut w = World::from_brushes(vec![slab(static_top)]);
    w.model_brushes.push(vec![slab(moving_top)]);
    w.model_origins.push(Vec3::ZERO);
    w.model_solid.push(true);
    w.update_models(&[Vec3::ZERO, Vec3::ZERO], &[true, true]);
    w
}
#[test]
fn world_floor_wins_over_a_lower_moving_support() {
    let w = world(0., -1.);
    assert_eq!(
        w.support_model(Vec3::Z * 0.03125, 72.),
        None,
        "a lower lift must not carry a player grounded on the world"
    );
}
#[test]
fn nearer_moving_support_is_carried() {
    let w = world(-1., 0.);
    assert_eq!(w.support_model(Vec3::Z * 0.03125, 72.), Some(1));
}
#[test]
fn coplanar_world_support_is_not_reassigned_to_a_train() {
    let w = world(0., 0.);
    assert_eq!(w.support_model(Vec3::Z * 0.03125, 72.), None);
}
