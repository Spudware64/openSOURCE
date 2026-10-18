use glam::Vec3;
use opensource::session::GameRoot;

#[test]
fn direct_loaded_escape_02_fires_on_authored_bts_concrete() {
    let root = GameRoot::new("../portal-original/portal", 4).unwrap();
    let mut loaded = root.load_map("escape_02", true).unwrap();
    assert_eq!(loaded.runtime.portal_inventory, [true, true]);
    let eye = loaded.player.position + Vec3::Z * loaded.player.eye_height();
    let direction = loaded.player.forward();
    let portal = loaded
        .game
        .fire_player_with_facing(
            0,
            eye,
            direction,
            direction,
            &loaded.runtime,
            &mut loaded.world,
        )
        .unwrap();
    assert_eq!(loaded.world.portals[0].unwrap().center, portal.center);
}

#[test]
fn escape_direct_loadout_preserves_early_chamber_restrictions() {
    let root = GameRoot::new("../portal-original/portal", 4).unwrap();
    for (map, expected) in [
        ("escape_00", [true, true]),
        ("escape_01", [true, true]),
        ("escape_02", [true, true]),
        ("testchmb_a_01", [false, false]),
        ("testchmb_a_03", [true, false]),
    ] {
        let loaded = root.load_map(map, true).unwrap();
        assert_eq!(loaded.runtime.portal_inventory, expected, "{map}");
    }
}
