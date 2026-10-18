fn main() -> Result<(), String> {
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", None)?;
    let script = fs.read("scripts/weapon_portalgun.txt")?;
    println!("weapon script:\n{}", String::from_utf8_lossy(&script));
    let bytes = fs.read("scenes/scenes.image")?;
    let durations = opensource::scenes::durations(&bytes)?;
    println!(
        "VSIF v2 image: {} bytes, {} scene duration records",
        bytes.len(),
        durations.len()
    );
    for path in [
        "scenes/test_chamber/ai_01_a_entry.vcd",
        "scenes/test_chamber/ai_01_a_portalgun.vcd",
    ] {
        let crc = opensource::scenes::scene_crc(path);
        println!(
            "{path}: CRC32 {crc:08x}; duration {:?} seconds",
            durations.get(&crc)
        );
    }
    Ok(())
}
