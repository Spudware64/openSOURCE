use glam::Vec3;
use portal_bsp_viewer::{
    entity::{parse_entities, Runtime},
    play::{Player, World},
};
#[test]
fn thrown_cube_transfers_momentum_to_another_runtime_cube() {
    let es = parse_entities(br#"{"classname" "prop_physics" "targetname" "first" "origin" "10 0 64"}{"classname" "prop_physics" "targetname" "second" "origin" "65 0 64"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    let p = Player::new(Vec3::new(-54., 0., 0.), 0.);
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    let mut p = p;
    for _ in 0..18 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[2].x > 70.,
        "second cube must receive a rigid-body impulse: {:?}",
        r.origins
    );
}

#[test]
fn linked_portals_keep_runtime_cube_impulses_active() {
    use portal_bsp_viewer::portal::Portal;
    let es = parse_entities(br#"{"classname" "prop_physics" "origin" "10 0 64"}{"classname" "prop_physics" "origin" "65 0 64"}"#).unwrap();
    let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
    let mut w = World::from_brushes(vec![]);
    w.portals = [
        Some(Portal::new(Vec3::new(-100., 0., 64.), Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(300., 0., 64.), -Vec3::X, Vec3::Z)),
    ];
    let mut p = Player::new(Vec3::new(-54., 0., 0.), 0.);
    assert!(r.use_player(&p, &w));
    assert!(r.throw_held(&p));
    for _ in 0..18 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[2].x > 70.,
        "linked scenes must retain Rapier impulses: {:?}",
        r.origins
    );
}
