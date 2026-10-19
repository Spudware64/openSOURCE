use glam::Vec3;
use opensource::{
    entity::Runtime,
    play::World,
    portal::{Game, Surface, Surfaces},
};
fn setup() -> (Game, Runtime, World) {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut game = Game::load(&bytes).unwrap();
    game.portals = [None; 2];
    game.automatic_surfaces = Some(Surfaces {
        faces: vec![Surface {
            points: vec![
                Vec3::new(600., -500., -500.),
                Vec3::new(600., 500., -500.),
                Vec3::new(600., 500., 500.),
                Vec3::new(600., -500., 500.),
            ],
            normal: -Vec3::X,
            allowed: true,
            material: "concrete".into(),
        }],
    });
    let mut runtime = Runtime::new(&bytes).unwrap();
    runtime.portal_inventory = [true; 2];
    (game, runtime, World::from_brushes(vec![]))
}
#[test]
fn rejected_shot_cannot_retarget_after_launch() {
    let (mut game, runtime, mut world) = setup();
    game.automatic_surfaces.as_mut().unwrap().faces[0].allowed = false;
    game.launch_player(
        0,
        Vec3::ZERO,
        Vec3::X,
        Vec3::X,
        Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    game.automatic_surfaces.as_mut().unwrap().faces[0].allowed = true;
    game.advance_shots(1., &runtime, &mut world);
    assert!(game.portals[0].is_none());
}
#[test]
fn simultaneous_arrivals_resolve_in_flight_time_order_not_click_order() {
    let (mut game, runtime, mut world) = setup();
    game.launch_player(
        0,
        Vec3::ZERO,
        Vec3::X,
        Vec3::X,
        Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    let near = Vec3::new(500., 200., 0.);
    game.launch_player(
        0,
        near,
        Vec3::X,
        Vec3::X,
        near + Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    assert!(game.shots[0].duration > game.shots[1].duration);
    game.advance_shots(1., &runtime, &mut world);
    assert!(
        game.portals[0].unwrap().center.y.abs() < 0.1,
        "far bolt arrives last and owns channel"
    );
}
#[test]
fn short_shot_trail_never_extends_behind_muzzle() {
    let (mut game, runtime, mut world) = setup();
    game.automatic_surfaces.as_mut().unwrap().faces[0]
        .points
        .iter_mut()
        .for_each(|p| p.x = 30.);
    game.launch_player(
        0,
        Vec3::ZERO,
        Vec3::X,
        Vec3::X,
        Vec3::new(20., 0., 0.),
        &runtime,
        &world,
    )
    .unwrap();
    game.advance_shots(0.059, &runtime, &mut world);
    assert!(game
        .shot_vertices(Vec3::new(0., -100., 0.))
        .iter()
        .all(|v| v.position[0] >= 19.99));
}
#[test]
fn arrival_emits_fading_impact_and_reset_cancels_pending_shots() {
    let (mut game, runtime, mut world) = setup();
    game.launch_player(
        0,
        Vec3::ZERO,
        Vec3::X,
        Vec3::X,
        Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    game.advance_shots(0.3, &runtime, &mut world);
    assert!(
        !game.shot_vertices(Vec3::ZERO).is_empty(),
        "arrival needs an impact bloom"
    );
    game.advance_shots(0.5, &runtime, &mut world);
    assert!(game.shot_vertices(Vec3::ZERO).is_empty());
    game.launch_player(
        1,
        Vec3::new(0., 200., 0.),
        Vec3::X,
        Vec3::X,
        Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    game.cancel_shots();
    game.advance_shots(5., &runtime, &mut world);
    assert!(game.portals[1].is_none());
    assert!(game.shot_vertices(Vec3::ZERO).is_empty());
}
#[test]
fn travelling_effect_has_channel_coloured_trail_and_bright_core() {
    let (mut game, runtime, _) = setup();
    let world = World::from_brushes(vec![]);
    for channel in 0..2 {
        game.launch_player(
            channel,
            Vec3::ZERO,
            Vec3::X,
            Vec3::X,
            Vec3::new(20., -8., -6.),
            &runtime,
            &world,
        )
        .unwrap();
    }
    let vertices = game.shot_vertices(Vec3::new(-50., -100., 40.));
    assert!(!vertices.is_empty());
    assert!(vertices.iter().any(|v| v.color[2] > v.color[0] * 2.));
    assert!(vertices.iter().any(|v| v.color[0] > v.color[2] * 2.));
    assert!(vertices.iter().any(|v| v.color.iter().all(|c| *c >= 0.8)));
    assert!(
        vertices.iter().any(|v| v.color == [0.; 3]),
        "additive glow needs transparent-black feathered edges"
    );
    assert!(vertices.iter().all(|v| Vec3::from(v.position).is_finite()));
}
#[test]
fn rejected_surface_still_emits_a_bolt_but_never_places() {
    let (mut game, runtime, mut world) = setup();
    game.automatic_surfaces.as_mut().unwrap().faces[0].allowed = false;
    game.launch_player(
        0,
        Vec3::ZERO,
        Vec3::X,
        Vec3::X,
        Vec3::X * 20.,
        &runtime,
        &world,
    )
    .unwrap();
    assert_eq!(game.shots.len(), 1);
    assert!(
        (game.shots[0].target.x - 600.).abs() < 0.1,
        "invalid material must impact its visible surface"
    );
    game.advance_shots(10., &runtime, &mut world);
    assert!(game.portals.iter().all(Option::is_none));
    assert!(game.shots.is_empty());
}
#[test]
fn player_shot_travels_from_muzzle_before_placing_portal() {
    let (mut game, runtime, mut world) = setup();
    let muzzle = Vec3::new(20., -8., -6.);
    game.launch_player(0, Vec3::ZERO, Vec3::X, Vec3::X, muzzle, &runtime, &world)
        .unwrap();
    assert!(game.portals[0].is_none());
    assert_eq!(game.shots[0].position(), muzzle);
    game.advance_shots(0.05, &runtime, &mut world);
    assert!(game.portals[0].is_none());
    assert!(game.shots[0].position().x > muzzle.x);
    game.advance_shots(1., &runtime, &mut world);
    assert!(game.portals[0].is_some());
    assert_eq!(
        game.portals[0].unwrap().center,
        world.portals[0].unwrap().center
    );
    assert!(game.shots.is_empty());
}
