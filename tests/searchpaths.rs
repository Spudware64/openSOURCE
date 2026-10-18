use opensource::assets::*;
#[test]
fn gameinfo_resolves_ordered_search_paths_without_portal_constants() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    assert!(fs
        .read("materials/concrete/concrete_modular_wall001a.vmt")
        .is_ok());
    assert!(fs.read("maps/testchmb_a_00.bsp").is_ok());
    let paths = fs.search_paths();
    assert!(paths[0].contains("portal_pak_dir.vpk"));
    assert!(paths.iter().any(|p| p.contains("hl2_textures_dir.vpk")));
}
