use opensource::session::GameRoot;

#[test]
fn native_and_prefetched_maps_keep_the_same_simulation() {
    let game = "../portal-original/portal";
    let map = "testchmb_a_02";
    let mut fs = opensource::assets::Assets::from_game(game, None).unwrap();
    let bytes = opensource::assets::read_selected_map(&fs, map).unwrap();
    fs = opensource::assets::Assets::from_game(game, Some(&bytes)).unwrap();
    fs.record_reads();
    opensource::scene::Scene::load_with_workers(&bytes, &fs, 1).unwrap();
    opensource::portal::Surfaces::load(&bytes, &fs).unwrap();
    let mut files = fs.recorded_files();
    files.push((format!("maps/{map}.bsp"), bytes));
    let mut native = GameRoot::new(game, 1).unwrap().load_map(map, true).unwrap();
    let mut memory = GameRoot::from_memory(files).unwrap().load_map(map, true).unwrap();
    for _ in 0..240 {
        for state in [&mut native, &mut memory] {
            state.runtime.tick(1./120., &mut state.player, &mut state.world);
            state.game.advance_shots(1./120., &state.runtime, &mut state.world);
            state.game.sync_runtime(&mut state.runtime, &mut state.world);
            state.player.step(&state.world, glam::Vec3::ZERO, false, 1./120.);
        }
        assert_eq!(native.player.position, memory.player.position);
        assert_eq!(native.player.velocity, memory.player.velocity);
    }
    println!("native and memory chamber02 after 240 ticks: {:?}", native.player.position);
}

#[test]
fn prefetched_game_root_rejects_unsafe_paths() {
    assert!(GameRoot::from_memory(vec![("../bad".into(), vec![])]).is_err());
}
#[test]
fn prefetched_game_root_lists_maps_and_reports_missing_map() {
    let root = GameRoot::from_memory(vec![("maps/test.bsp".into(), vec![])]).unwrap();
    assert_eq!(root.maps().unwrap(), vec!["test"]);
    assert!(root.load_map("missing", true).is_err());
}
