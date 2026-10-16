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
