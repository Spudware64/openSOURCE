use opensource::play::*;
#[test]
fn real_brush_models_are_transformed_and_can_move_collision() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut w = World::parse(&b).unwrap();
    assert_eq!(w.model_origins[14], glam::Vec3::new(-484., -720., 184.));
    assert!(!w.model_brushes[14].is_empty());
    let old = w.brushes.clone();
    let mut origins = w.model_origins.clone();
    origins[14].x += 100.;
    w.update_models(&origins, &vec![true; origins.len()]);
    assert_ne!(format!("{:?}", old), format!("{:?}", w.brushes));
}
