use portal_bsp_viewer::*;
#[test]
fn real_background_geometry() {
    let b = std::fs::read("../portal-original/portal/maps/background2.bsp").unwrap();
    let m = load(&b).unwrap();
    assert!(m.len() > 100);
    assert!(m.iter().all(|v| v.position.iter().all(|x| x.is_finite())));
    let mut bad = b.clone();
    let off = i32::from_le_bytes(b[216..220].try_into().unwrap()) as usize;
    bad[off..off + 4].copy_from_slice(&i32::MIN.to_le_bytes());
    assert!(load(&bad).is_err());
}
