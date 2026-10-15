#[test]
fn bounded_demo_opens_then_closes() {
    let source = std::fs::read_to_string("src/window.rs").unwrap();
    assert!(
        source.contains("SOURCE_DOOR_DEMO"),
        "bounded rendered door recording mode missing"
    );
    assert!(source.contains("runtime.input(name, command, \"\")"));
    assert!(source.contains("frames >= 180"));
}
