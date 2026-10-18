#[test]
fn interactive_entry_point_is_available() {
    let _entry: fn(&[u8]) -> Result<(), String> = opensource::interactive;
}
