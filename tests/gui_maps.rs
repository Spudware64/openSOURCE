use opensource::session::GameRoot;

#[test]
fn selected_map_loads_at_its_authored_spawn_and_rejects_paths() {
    let root = GameRoot::new("../portal-original/portal", 4).unwrap();
    assert!(root.maps().unwrap().contains(&"testchmb_a_00".to_owned()));
    assert!(root.load_map("../testchmb_a_00", false).is_err());
    assert!(root.load_map("testchmb_a_00.bsp", false).is_err());
    let loaded = root.load_map("testchmb_a_00", false).unwrap();
    assert_eq!(loaded.player.position, loaded.world.spawn);
    assert!(loaded.game.portals.iter().all(Option::is_none));
    assert!(!loaded.scene.vertices.is_empty());
}
