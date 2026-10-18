use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Plane, Player, World},
    portal::Portal,
};
fn fixture() -> (Runtime, World, Player) {
    let es = parse_entities(br#"{"classname" "prop_physics" "origin" "40 0 64"}"#).unwrap();
    let r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![
        vec![
            Plane {
                normal: Vec3::X,
                distance: 110.,
            },
            Plane {
                normal: -Vec3::X,
                distance: -90.,
            },
        ],
        vec![Plane {
            normal: Vec3::X,
            distance: 0.,
        }],
        vec![Plane {
            normal: -Vec3::X,
            distance: -200.,
        }],
    ]);
    w.portals = [
        Some(Portal::new(Vec3::new(0., 0., 64.), Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(200., 0., 64.), -Vec3::X, Vec3::Z)),
    ];
    let mut p = Player::new(Vec3::new(80., 0., 0.), std::f32::consts::PI);
    p.pitch = 0.;
    (r, w, p)
}
#[test]
fn thrown_cube_crosses_solid_portal_wall_and_updates_collider() {
    let (mut r, mut w, mut p) = fixture();
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    for _ in 0..20 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.visible[1]);
    assert!(r.origins[1].x > 100., "{:?}", r.origins[1]);
    assert!(
        w.overlaps_cuboid(r.origins[1], Vec3::splat(1.)),
        "collider follows model"
    );
}
#[test]
fn linked_portal_scene_keeps_ordinary_wall_contacts() {
    let (mut r, mut w, mut p) = fixture();
    // Pair remains linked, but the thrown cube is outside its opening.
    for portal in w.portals.iter_mut().flatten() {
        portal.center.y = 100.;
    }
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[1].x >= 19. && r.origins[1].x < 100.,
        "ordinary wall leaked: {:?}",
        r.origins[1]
    );
}

#[test]
fn thrown_cube_rejects_blocked_destination() {
    let (mut r, mut w, mut p) = fixture();
    // A separate solid occupies the destination aperture, not its backing wall.
    w.brushes.push(vec![
        Plane {
            normal: Vec3::X,
            distance: 205.,
        },
        Plane {
            normal: -Vec3::X,
            distance: -175.,
        },
    ]);
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[1].x >= 19. && r.origins[1].x < 100.,
        "blocked exit leaked: {:?}",
        r.origins[1]
    );
}

#[test]
fn carried_cube_crosses_solid_portal_wall() {
    let (mut r, mut w, mut p) = fixture();
    assert!(r.use_player(&p, &w));
    p.position.x = 30.;
    for _ in 0..2 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(r.visible[1]);
    assert!(
        r.origins[1].x > 100.,
        "held cube must map, not enter source wall: {:?}",
        r.origins[1]
    );
}

#[test]
fn carried_cube_retracts_through_portal_not_across_intervening_world() {
    let (mut r, mut w, mut p) = fixture();
    assert!(r.use_player(&p, &w));
    p.position.x = 30.;
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[1].x > 150.);
    }
    p.position.x = 80.;
    for tick in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(
            r.origins[1].x < 40.,
            "tick {tick}: retracted cube must return via source, not sweep across world: {:?}",
            r.origins[1]
        );
    }
}
#[test]
fn carried_cube_never_crosses_ordinary_wall_over_many_ticks() {
    let (mut r, mut w, mut p) = fixture();
    w.portals = [None; 2];
    assert!(r.use_player(&p, &w));
    p.position.x = 30.;
    for tick in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[1].x >= 16., "tick {tick}: {:?}", r.origins[1]);
    }
}

#[test]
fn carried_cube_rejects_blocked_destination_without_dragging_through_wall() {
    let (mut r, mut w, mut p) = fixture();
    w.brushes.push(vec![
        Plane { normal: Vec3::X, distance: 205. },
        Plane { normal: -Vec3::X, distance: -175. },
    ]);
    assert!(r.use_player(&p, &w));
    p.position.x = 30.;
    for tick in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[1].x >= 0. && r.origins[1].x < 90.,
            "blocked carry tick {tick}: {:?}", r.origins[1]);
    }
}

#[test]
fn carried_cube_pair_fizzle_clears_mapping_without_world_wall_leak() {
    let (mut r, mut w, mut p) = fixture();
    assert!(r.use_player(&p, &w));
    p.position.x = 30.;
    r.tick(1. / 120., &mut p, &mut w);
    assert!(r.origins[1].x > 150.);
    let pair = w.portals;
    w.portals = [None; 2];
    p.position.x = 80.;
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[1].x > 110., "fizzle leaked through intervening wall");
    }
    w.portals = pair;
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.origins[1].x > 110., "reactivation reused stale carry mapping");
    }
}
