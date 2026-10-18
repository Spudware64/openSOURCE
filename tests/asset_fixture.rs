use opensource::assets::{read_selected_map, Assets};

#[test]
fn discovered_map_selector_reaches_startup_asset_lookup() {
    let root = std::env::temp_dir().join(format!("source-map-selector-{}", std::process::id()));
    let game = root.join("portal");
    std::fs::create_dir_all(game.join("maps")).unwrap();
    std::fs::write(game.join("gameinfo.txt"), r#""GameInfo"{"FileSystem"{"SearchPaths"{"game" "|gameinfo_path|."}}}"#).unwrap();
    std::fs::write(game.join("maps/escape_02.bsp"), b"fixture bsp").unwrap();
    let fs = Assets::from_game(&game, None).unwrap();
    for selector in fs.maps() {
        assert_eq!(read_selected_map(&fs, &selector).unwrap(), b"fixture bsp");
        assert_eq!(read_selected_map(&fs, &format!("{selector}.bsp")).unwrap(), b"fixture bsp");
        assert_eq!(read_selected_map(&fs, &format!("maps/{selector}.bsp")).unwrap(), b"fixture bsp");
    }
    assert!(read_selected_map(&fs, "../escape_02").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
fn inline_vpk() -> Vec<u8> {
    let mut tree = b"txt\0 \0test\0".to_vec();
    tree.extend(0u32.to_le_bytes());
    tree.extend(3u16.to_le_bytes());
    tree.extend(0x7fffu16.to_le_bytes());
    tree.extend(0u32.to_le_bytes());
    tree.extend(4u32.to_le_bytes());
    tree.extend(0xffffu16.to_le_bytes());
    tree.extend(b"pre\0\0\0");
    let mut b = vec![];
    b.extend(0x55aa1234u32.to_le_bytes());
    b.extend(1u32.to_le_bytes());
    b.extend((tree.len() as u32).to_le_bytes());
    b.extend(tree);
    b.extend(b"body");
    b
}
#[test]
fn renamed_game_fixture_custom_vpk_preload_and_shared_paths() {
    let root = std::env::temp_dir().join(format!("source-searchpaths-{}", std::process::id()));
    let game = root.join("my_mod");
    std::fs::create_dir_all(game.join("custom/a")).unwrap();
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(game.join("gameinfo.txt"),r#""GameInfo"{"FileSystem"{"SearchPaths"{"game" "my_mod/custom/*" "game" "|gameinfo_path|pack.vpk" "mod" "|gameinfo_path|." "game" "|all_source_engine_paths|shared"}}}"#).unwrap();
    std::fs::write(game.join("custom/a/test.txt"), b"custom").unwrap();
    std::fs::write(game.join("pack.vpk"), inline_vpk()).unwrap();
    std::fs::write(root.join("shared/other.txt"), b"shared").unwrap();
    let fs = Assets::from_game(&game, None).unwrap();
    assert_eq!(fs.read("TEST.TXT").unwrap(), b"custom");
    assert_eq!(fs.read("other.txt").unwrap(), b"shared");
    std::fs::remove_dir_all(game.join("custom")).unwrap();
    let fs = Assets::from_game(&game, None).unwrap();
    assert_eq!(fs.read("test.txt").unwrap(), b"prebody");
    std::fs::write(game.join("pack.vpk"), &inline_vpk()[..20]).unwrap();
    assert!(Assets::from_game(&game, None).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
