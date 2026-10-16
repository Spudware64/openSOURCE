use glam::Vec3;
use portal_bsp_viewer::play::{Plane, Player, World};
#[test]
fn default_player_uses_parry_narrowphase() {
    let w = World::from_brushes(vec![vec![Plane {
        normal: Vec3::Z,
        distance: 0.,
    }]]);
    let mut p = Player::new(Vec3::new(0., 0., 0.015625), 0.);
    let before = w.player_narrowphase_queries();
    for _ in 0..120 {
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert!(p.grounded);
    assert!(p.position.z >= 0. && p.position.z < 0.032);
    assert!(w.player_narrowphase_queries() > before);
}
