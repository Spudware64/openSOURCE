use glam::Vec3;
use opensource::{assets::Assets, portal::Portal, render_portal_scene, scene::Scene};
#[test]
fn unlinked_portal_is_visibly_dormant_not_a_flat_remote_view_placeholder() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let portal = Portal::new(Vec3::new(-832., -320., 180.), Vec3::X, Vec3::Z);
    let pixels = render_portal_scene(&scene, 256, 192,
        Some(([-740., -320., 192.], [-832., -320., 192.])), [Some(portal), None]).unwrap();
    if let Ok(path) = std::env::var("SOURCE_DORMANT_CAPTURE") {
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 256, 192);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&pixels).unwrap();
    }
    let mut colors = std::collections::HashSet::new();
    for y in 96..116 { for x in 118..138 {
        colors.insert(pixels[(y*256+x)*4..(y*256+x)*4+3].to_vec());
    }}
    assert!(colors.len() > 8, "dormant aperture was flat: {} colors", colors.len());
}
