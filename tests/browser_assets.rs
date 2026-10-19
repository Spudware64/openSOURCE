use opensource::assets::Assets;

#[test]
fn memory_assets_normalize_and_reject_traversal() {
    let fs = Assets::from_memory(vec![("Materials\\Test.VMT".into(), b"material".to_vec())], None).unwrap();
    assert_eq!(fs.read("materials/test.vmt").unwrap(), b"material");
    assert!(Assets::from_memory(vec![("../escape".into(), vec![])], None).is_err());
    assert!(fs.read("missing").is_err());
}

#[test]
fn memory_assets_enumerate_only_valid_maps() {
    let fs = Assets::from_memory(vec![("maps/test.bsp".into(), vec![]), ("maps/bad-name.bsp".into(), vec![])], None).unwrap();
    assert_eq!(fs.maps(), vec!["test"]);
}
