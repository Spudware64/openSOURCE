use portal_bsp_viewer::*;
#[test]
fn metal_renders_real_map() {
    let b = std::fs::read("../portal-original/portal/maps/background2.bsp").unwrap();
    let v = load(&b).unwrap();
    let pixels = render(&v, 256, 256, None).unwrap();
    assert_eq!(pixels.len(), 256 * 256 * 4);
    let distinct: std::collections::HashSet<_> = pixels.chunks_exact(4).collect();
    assert!(
        distinct.len() > 8,
        "expected visible shaded map, got {} colors",
        distinct.len()
    );
}
