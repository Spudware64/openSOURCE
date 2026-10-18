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

#[test]
fn map_discovery_uses_game_search_paths_and_safe_names() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let maps = fs.maps();
    assert!(maps.iter().any(|name| name == "testchmb_a_00"));
    assert!(maps.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(map_name("testchmb_a_00.bsp"), Some("testchmb_a_00"));
    for bad in [
        "../escape.bsp",
        "nested/map.bsp",
        "bad-name.bsp",
        ".bsp",
        "map.txt",
    ] {
        assert_eq!(map_name(bad), None, "{bad}");
    }
}
