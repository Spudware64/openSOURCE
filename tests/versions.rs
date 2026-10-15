use portal_bsp_viewer::*;
#[test]
fn rejects_unknown_face_layout() {
    let mut b = std::fs::read("../portal-original/portal/maps/background2.bsp").unwrap();
    b[128..132].copy_from_slice(&99i32.to_le_bytes());
    assert!(load(&b).is_err());
}
