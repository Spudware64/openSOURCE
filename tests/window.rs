#[test]
fn interactive_entry_point_is_available() {
    let _entry: fn(&[u8]) -> Result<(), String> = portal_bsp_viewer::interactive;
}
