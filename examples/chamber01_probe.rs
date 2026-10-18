//! Headless counterpart to SOURCE_CHAMBER01_SMOKE, sharing its input-only route.
use opensource::{
    assets::Assets,
    chamber01_route::Route,
    entity::Runtime,
    play::{Player, World},
    portal::{Game, Surfaces},
    scene::Scene,
};
fn main() -> Result<(), String> {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp")
        .map_err(|e| e.to_string())?;
    let assets = Assets::from_game("../portal-original/portal", Some(&bytes))?;
    let scene = Scene::load(&bytes, &assets)?;
    let mut runtime = Runtime::new(&bytes)?;
    runtime.portal_events = Some(vec![]);
    runtime.bind_animations(&scene);
    let mut world = World::parse(&bytes)?;
    world.bind_scene(&scene);
    world.update_models(&runtime.origins, &runtime.visible);
    let mut player = Player::new(world.spawn, world.yaw);
    let mut game = Game::load(&bytes)?;
    game.automatic_surfaces = Some(Surfaces::load(&bytes, &assets)?);
    let mut route = Route::default();
    while !route.tick(&mut player, &mut runtime, &mut world, &mut game)? {}
    println!(
        "route completed in {} fixed ticks; player {:?}; unsupported {:?}",
        route.ticks, player.position, runtime.unknown_inputs
    );
    Ok(())
}
