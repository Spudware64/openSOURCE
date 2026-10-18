use opensource::assets::*;
#[test]
fn actual_archives_resolve_materials_without_extraction() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let material = fs.read("materials/metal/metalwall048a.vmt").unwrap();
    assert!(!material.is_empty());
    assert!(fs.read("../bin/engine.dll").is_err());
    assert!(fs.read("/bin/engine.dll").is_err());
    assert!(fs.read("materials/../../bin/engine.dll").is_err());
}
