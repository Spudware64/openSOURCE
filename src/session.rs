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
    pub fn audio(&self, bytes: &[u8]) -> Result<crate::audio::Audio, String> {
        Ok(crate::audio::Audio::start(Assets::from_game(
            &self.game,
            Some(bytes),
        )?))
    }
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
    pub fn maps(&self) -> Result<Vec<String>, String> {
        Ok(Assets::from_game(&self.game, None)?.maps())
    }
    pub fn load_map(&self, map: &str, portals: bool) -> Result<PreparedMap, String> {
        self.prepare_map(map, None, portals)
    }
    pub fn prepare(
        &self,
        request: &ChangeLevel,
        previous: &Player,
        portals: bool,
    ) -> Result<PreparedMap, String> {
        self.prepare_map(&request.map, Some((request, previous)), portals)
    }
    fn prepare_map(
        &self,
        map: &str,
        transition: Option<(&ChangeLevel, &Player)>,
        portals: bool,
    ) -> Result<PreparedMap, String> {
        if crate::assets::map_name(&format!("{map}.bsp")) != Some(map) {
            return Err("unsafe map name".into());
        }
        // Resolve the BSP without the old map's embedded pak, then mount only the new pak.
        let bytes = Assets::from_game(&self.game, None)?.read(&format!("maps/{}.bsp", map))?;
        let mut world = World::parse(&bytes)?;
        let position = if let Some((request, previous)) = transition {
            let entities = parse_entities(crate::lump(&bytes, 0, 1)?)?;
            request.arrival(&entities, previous.position)?
        } else {
            world.spawn
        };
        let fs = Assets::from_game(&self.game, Some(&bytes))?;
        let scene = Scene::load_with_workers(&bytes, &fs, self.workers)?;
        let mut runtime = Runtime::new(&bytes)?;
        runtime.bind_animations(&scene);
        world.bind_scene(&scene);
        let (origins, rotations) = runtime.model_transforms();
        world.update_transforms(&origins, &runtime.visible, &rotations);
        let mut player = Player::new(
            position,
            transition.map_or(world.yaw, |(_, previous)| previous.yaw),
        );
        if let Some((_, previous)) = transition {
            player.pitch = previous.pitch;
            player.velocity = previous.velocity;
            player.crouched = previous.crouched;
            world.spawn = position;
            world.yaw = previous.yaw;
        }
        let mut game = crate::portal::Game::load(&bytes)?;
        // Never carry portal linkage or held/template bodies across maps.
        game.portals = [None, None];
        let surfaces = if portals {
            runtime.portal_events = Some(vec![]);
            if transition.is_none() {
                runtime.direct_launch_loadout();
            }
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
