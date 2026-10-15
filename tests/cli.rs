#[test]
fn cli_writes_png() {
    let out = "evidence/cli-test.png";
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_portal-bsp-viewer"))
        .args(["../portal-original/portal/maps/background2.bsp", out])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(out).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
}
