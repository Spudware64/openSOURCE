#[test]
fn generic_game_root_cli_loads_textured_map_and_rejects_worker_explosion() {
    let output = std::env::temp_dir().join(format!("source-textured-{}.png", std::process::id()));
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_openSOURCE"))
        .args([
            "--game",
            "../portal-original/portal",
            "--map",
            "maps/background2.bsp",
            "--workers",
            "2",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.is_file());
    std::fs::remove_file(output).unwrap();
    let bad = std::process::Command::new(env!("CARGO_BIN_EXE_openSOURCE"))
        .args([
            "--game",
            "../portal-original/portal",
            "--map",
            "maps/background2.bsp",
            "--workers",
            "99",
            "--output",
            "unused.png",
        ])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("1..16"));
}
