//! Browser entry point: feed the native parser/renderer/runtime a resolved asset ZIP.
use wasm_bindgen::prelude::*;
use std::io::Read;

#[wasm_bindgen]
pub async fn start_game(bundle: &[u8], map: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    start(bundle, map).await.map_err(|e| JsValue::from_str(&e))
}
async fn start(bundle: &[u8], map: &str) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bundle)).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).map_err(|e| e.to_string())?;
        if file.is_dir() { continue; }
        crate::assets::normalise(file.name())?;
        total = total.checked_add(file.size()).ok_or("asset ZIP size overflow")?;
        if file.size() > 64 * 1024 * 1024 || total > 512 * 1024 * 1024 { return Err("oversized browser bundle".into()); }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        files.push((file.name().to_string(), bytes));
    }
    let root = crate::session::GameRoot::from_memory(files)?;
    let bytes = crate::assets::read_selected_map(&root.assets(None)?, map)?;
    let assets = root.assets(Some(&bytes))?;
    let scene = crate::scene::Scene::load_with_workers(&bytes, &assets, 1)?;
    let surfaces = crate::portal::Surfaces::load(&bytes, &assets)?;
    web_sys::console::log_1(&format!("openSOURCE WASM: {map}, {} vertices, {} textures; shared native runtime; audio unavailable", scene.vertices.len(), scene.textures.len()).into());
    crate::window::interactive_async(&bytes, Some(scene), Some(surfaces), Some(root)).await
}
