use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::Runtime,
    play::{Player, World},
    portal::{Game, Surfaces},
};
#[test]
fn chamber00_authored_start_delay_then_normal_walk_crosses_first_pair() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut w = World::parse(&b).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut game = Game::load(&b).unwrap();
    game.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let mut p = Player::spawn(&w);
    for tick in 0..8100 {
        r.tick(1. / 120., &mut p, &mut w);
        game.sync_runtime(&mut r, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
        if tick == 120 {
            assert!(
                w.portals.iter().all(Option::is_none),
                "temporary start portals must fizzle after 0.5 seconds"
            );
        }
    }
    assert!(
        w.portals.iter().all(Option::is_some),
        "first real pair must activate after authored 66.5-second wait"
    );
    println!(
        "authored pair after wait {:?}; player {:?}",
        w.portals, p.position
    );
    p.yaw = 0.;
    for _ in 0..20 {
        r.tick(1. / 120., &mut p, &mut w);
        game.sync_runtime(&mut r, &mut w);
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    p.yaw = -std::f32::consts::FRAC_PI_2;
    for _ in 0..90 {
        r.tick(1. / 120., &mut p, &mut w);
        game.sync_runtime(&mut r, &mut w);
        p.step(&w, Vec3::X, false, 1. / 120.);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert!(
        p.portal_crossings > 0,
        "normal input must cross first authored portal pair: {:?}",
        p.position
    );
    println!(
        "first pair crossed without pose injection: {:?}",
        p.position
    );
}
