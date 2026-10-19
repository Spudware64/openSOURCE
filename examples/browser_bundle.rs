//! Private bundle of original bytes actually read by the existing scene loader.
//! Never commit the output; original installation remains read-only.
use std::{collections::BTreeMap, io::Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 { return Err("usage: browser_bundle GAME_ROOT OUTPUT.zip MAP [MAP ...]".into()); }
    let game = &args[1];
    let base = opensource::assets::Assets::from_game(game, None)?;
    let mut files = BTreeMap::new();
    for map in &args[3..] {
        let bytes = opensource::assets::read_selected_map(&base, map)?;
        let mut fs = opensource::assets::Assets::from_game(game, Some(&bytes))?;
        fs.record_reads();
        let scene = opensource::scene::Scene::load_with_workers(&bytes, &fs, 2)?;
        opensource::portal::Surfaces::load(&bytes, &fs)?;
        opensource::scene::Scene::weapon_viewmodel(&fs, "weapon_portalgun")?;
        eprintln!("{map}: {} vertices, {} warnings", scene.vertices.len(), scene.warnings.len());
        files.extend(fs.recorded_files());
        files.insert(format!("maps/{map}.bsp"), bytes);
    }
    let file = std::fs::File::create(&args[2])?;
    let mut zip = zip::ZipWriter::new(file);
    let count = files.len();
    let total: usize = files.values().map(Vec::len).sum();
    for (name, bytes) in files {
        zip.start_file(name, zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated))?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?;
    println!("Private browser bundle: {count} files, {total} uncompressed bytes -> {}", args[2]);
    Ok(())
}
