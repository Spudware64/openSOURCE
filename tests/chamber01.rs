use opensource::{
    entity::Runtime,
    play::{Player, World},
};
#[test]
fn actual_rotating_pedestal_completes_authored_open_close_cycle() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    r.input("tca01_rotate_launcher_door1", "Open", "");
    for _ in 0..370 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        !r.unknown_inputs
            .iter()
            .any(|s| s.starts_with("func_door_rotating:Open")),
        "{:?}",
        r.unknown_inputs
    );
    // The authored open completion fires the relay, which charges and fires again.
    assert!(
        !r.unknown_inputs
            .iter()
            .any(|s| s.starts_with("func_door_rotating:Close")),
        "{:?}",
        r.unknown_inputs
    );
    assert!(p.position.is_finite());
}
#[test]
fn authored_relay_fires_automatic_portal_at_current_parented_aim() {
    use opensource::{
        assets::Assets,
        portal::{Game, Surfaces},
    };
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let surfaces = Surfaces::load(&b, &fs).unwrap();
    let mut game = Game::load(&b).unwrap();
    game.automatic_surfaces = Some(surfaces);
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(w.spawn, w.yaw);
    for _ in 0..2400 {
        r.tick(1. / 120., &mut p, &mut w);
        game.sync_runtime(&mut r, &mut w);
    }
    assert!(
        game.portals[0].is_some(),
        "automatic first blue portal missing: {:?}",
        r.unknown_inputs
    );
}
