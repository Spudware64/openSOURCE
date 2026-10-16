//! Generic game-root retention and transactional Source map preparation.
use crate::{
    assets::Assets,
    entity::{parse_entities, ChangeLevel, Runtime},
    play::{Player, World},
    scene::Scene,
};
use std::path::{Path, PathBuf};
pub struct GameRoot {
    game: PathBuf,
    workers: usize,
}
pub struct PreparedMap {
    pub scene: Scene,
    pub world: World,
    pub runtime: Runtime,
    pub player: Player,
    pub game: crate::portal::Game,
    pub surfaces: Option<crate::portal::Surfaces>,
}
impl GameRoot {
    pub fn weapon_viewmodel(&self) -> Result<Scene, String> {
        Scene::weapon_viewmodel(&Assets::from_game(&self.game, None)?, "weapon_portalgun")
    }
    pub fn new(game: impl AsRef<Path>, workers: usize) -> Result<Self, String> {
        if !(1..=16).contains(&workers) {
            return Err("asset workers must be 1..16".into());
        }
        let game = game.as_ref().canonicalize().map_err(|e| e.to_string())?;
        Assets::from_game(&game, None)?;
        Ok(Self { game, workers })
    }
    pub fn prepare(
        &self,
        request: &ChangeLevel,
        previous: &Player,
        portals: bool,
    ) -> Result<PreparedMap, String> {
        if request.map.is_empty()
            || !request
                .map
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err("unsafe changelevel map".into());
        }
        // Resolve the BSP without the old map's embedded pak, then mount only the new pak.
        let bytes =
            Assets::from_game(&self.game, None)?.read(&format!("maps/{}.bsp", request.map))?;
        let entities = parse_entities(crate::lump(&bytes, 0, 1)?)?;
        let position = request.arrival(&entities, previous.position)?;
        let fs = Assets::from_game(&self.game, Some(&bytes))?;
        let scene = Scene::load_with_workers(&bytes, &fs, self.workers)?;
        let mut world = World::parse(&bytes)?;
        let mut runtime = Runtime::new(&bytes)?;
        runtime.bind_animations(&scene);
        world.bind_scene(&scene);
        world.update_models(&runtime.origins, &runtime.visible);
        let mut player = Player::new(position, previous.yaw);
        player.pitch = previous.pitch;
        player.velocity = previous.velocity;
        player.crouched = previous.crouched;
        world.spawn = position;
        world.yaw = previous.yaw;
        let mut game = crate::portal::Game::load(&bytes)?;
        // Never carry portal linkage or held/template bodies across maps.
        game.portals = [None, None];
        let surfaces = if portals {
            runtime.portal_events = Some(vec![]);
            Some(crate::portal::Surfaces::load(&bytes, &fs)?)
        } else {
            None
        };
        game.automatic_surfaces = surfaces.clone();
        Ok(PreparedMap {
            scene,
            world,
            runtime,
            player,
            game,
            surfaces,
        })
    }
}
