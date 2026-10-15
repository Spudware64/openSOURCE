use glam::Vec3;
use portal_bsp_viewer::play::*;
fn box_brush(lo: Vec3, hi: Vec3) -> Vec<Plane> {
    vec![
        Plane {
            normal: Vec3::X,
            distance: hi.x,
        },
        Plane {
            normal: -Vec3::X,
            distance: -lo.x,
        },
        Plane {
            normal: Vec3::Y,
            distance: hi.y,
        },
        Plane {
            normal: -Vec3::Y,
            distance: -lo.y,
        },
        Plane {
            normal: Vec3::Z,
            distance: hi.z,
        },
        Plane {
            normal: -Vec3::Z,
            distance: -lo.z,
        },
    ]
}
#[test]
fn walking_steps_onto_eighteen_unit_ledge_but_not_tall_wall() {
    for height in [16., 30.] {
        let world = World::from_brushes(vec![
            box_brush(Vec3::new(-500., -100., -20.), Vec3::new(500., 100., 0.)),
            box_brush(Vec3::new(40., -100., 0.), Vec3::new(200., 100., height)),
        ]);
        let mut p = Player::new(Vec3::new(0., 0., 0.1), 0.);
        for _ in 0..120 {
            p.step(&world, Vec3::X, false, 1. / 120.);
        }
        if height == 16. {
            assert!(p.position.x > 100., "{:?}", p.position);
            assert!((p.position.z - height).abs() < 0.1);
        } else {
            assert!(p.position.x < 25.);
        }
    }
}
#[test]
fn upward_launch_near_floor_is_not_ground_or_a_second_jump() {
    let world = World::from_brushes(vec![box_brush(
        Vec3::new(-500., -100., -20.),
        Vec3::new(500., 100., 0.),
    )]);
    for (speed, expected) in [(140., true), (141., false)] {
        let mut launched = Player::new(Vec3::new(0., 0., 0.1), 0.);
        launched.velocity.z = speed;
        launched.step(&world, Vec3::ZERO, false, 1. / 120.);
        assert_eq!(launched.grounded, expected, "upward speed {speed}");
    }
    let mut p = Player::new(Vec3::new(0., 0., 0.1), 0.);
    p.velocity = Vec3::new(0., 0., 400.);
    p.step(&world, Vec3::ZERO, true, 1. / 120.);
    assert!(
        !p.grounded,
        "a nearby floor must not ground an upward launch"
    );
    assert!(
        (p.velocity.z - 395.).abs() < 0.01,
        "held jump must not replace launch velocity: {:?}",
        p.velocity
    );
}
#[test]
fn crouch_fits_low_ceiling_and_cannot_stand_inside_it() {
    let world = World::from_brushes(vec![
        box_brush(Vec3::new(-500., -100., -20.), Vec3::new(500., 100., 0.)),
        box_brush(Vec3::new(40., -100., 45.), Vec3::new(200., 100., 90.)),
    ]);
    let mut p = Player::new(Vec3::new(0., 0., 0.1), 0.);
    p.set_crouch(&world, true);
    assert!(p.crouched);
    for _ in 0..120 {
        p.step(&world, Vec3::X, false, 1. / 120.);
    }
    assert!(p.position.x > 60.);
    p.set_crouch(&world, false);
    assert!(p.crouched, "standing would intersect ceiling");
    p.position.x = 250.;
    p.set_crouch(&world, false);
    assert!(!p.crouched);
}
