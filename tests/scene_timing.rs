use portal_bsp_viewer::{
    entity::Runtime,
    play::{Player, World},
};
#[test]
fn actual_entry_scene_completes_before_authored_entrance_opens() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    let fs = portal_bsp_viewer::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let durations =
        portal_bsp_viewer::scenes::durations(&fs.read("scenes/scenes.image").unwrap()).unwrap();
    assert_eq!(
        durations[&portal_bsp_viewer::scenes::scene_crc("scenes/test_chamber/ai_01_a_entry.vcd")],
        21.039
    );
    r.bind_scene_timing(&durations);
    r.input("choreo_AI_01_A_entry", "Start", "");
    for _ in 0..2400 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert_eq!(r.origins[8].x, 466.);
    for _ in 0..240 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.origins[8].x > 480.,
        "scene completion must fire actual door outputs: {:?}",
        r.unknown_inputs
    );
}
