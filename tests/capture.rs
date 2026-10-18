#[test]
fn surface_bgra_capture_preserves_rgb() {
    assert_eq!(
        opensource::surface_rgba(&[20, 40, 80, 255], true),
        vec![80, 40, 20, 255]
    );
}
