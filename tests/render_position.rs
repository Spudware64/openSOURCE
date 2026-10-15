use glam::Vec3;
use portal_bsp_viewer::play::{Player, RenderPosition};
#[test]
fn camera_moves_between_ticks_but_snaps_portal_discontinuities() {
    let mut p = Player::new(Vec3::ZERO, 0.);
    let mut render = RenderPosition::default();
    render.begin_step(&p);
    p.position = Vec3::X * 10.;
    assert_eq!(render.sample(&p, 0.25), Vec3::X * 2.5);
    assert_eq!(render.sample(&p, 0.75), Vec3::X * 7.5);
    p.portal_crossings += 1;
    p.position = Vec3::Y * 100.;
    assert_eq!(render.sample(&p, 0.5), p.position);
    render = RenderPosition::default();
    assert_eq!(render.sample(&p, 0.5), p.position);
}
