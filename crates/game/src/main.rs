mod objectives;
mod player;
mod save;
mod scenes;
mod ui;
mod world;

use anyhow::{Result, anyhow, bail};
use content::{
    CodexDatabase, CutsceneDatabase, DEFAULT_CODEX_DB_PATH, DEFAULT_CUTSCENE_DB_PATH,
    DEFAULT_EVENT_DB_PATH, EventDatabase, semantics,
};
use runtime::{Camera2d, Game, InputState, Renderer, run};
use save::{DEFAULT_SAVE_PATH, SaveData};
use scenes::{GameContext, SceneId, SceneStack};

struct AlienArchiveApp {
    scenes: SceneStack,
    context: GameContext,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct GameLaunchOptions {
    scene: Option<SceneId>,
    map_path: Option<String>,
    spawn_id: Option<String>,
}

impl GameLaunchOptions {
    fn from_env_and_args() -> Result<Self> {
        let mut options = Self::from_env()?;
        options.apply_args(std::env::args().skip(1))?;
        options.infer_scene();
        Ok(options)
    }

    fn from_env() -> Result<Self> {
        let scene = match non_empty_env("ALIEN_ARCHIVE_SCENE") {
            Some(value) => Some(parse_scene_id(&value)?),
            None => None,
        };

        Ok(Self {
            scene,
            map_path: non_empty_env("ALIEN_ARCHIVE_MAP"),
            spawn_id: non_empty_env("ALIEN_ARCHIVE_SPAWN"),
        })
    }

    #[cfg(test)]
    fn from_args<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut options = Self::default();
        options.apply_args(args)?;
        options.infer_scene();
        Ok(options)
    }

    fn apply_args<I, S>(&mut self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        while let Some(arg) = args.next() {
            if let Some((key, value)) = arg.split_once('=') {
                self.apply_arg_value(key, value)?;
                continue;
            }

            match arg.as_str() {
                "--map" => {
                    self.map_path = Some(required_arg_value(&mut args, "--map")?);
                }
                "--spawn" => {
                    self.spawn_id = Some(required_arg_value(&mut args, "--spawn")?);
                }
                "--scene" => {
                    let value = required_arg_value(&mut args, "--scene")?;
                    self.scene = Some(parse_scene_id(&value)?);
                }
                "--help" | "-h" => {
                    bail!(
                        "launch options: --map <path> [--spawn <id>] [--scene overworld|facility]"
                    );
                }
                _ => bail!("unknown launch option: {arg}"),
            }
        }

        Ok(())
    }

    fn apply_arg_value(&mut self, key: &str, value: &str) -> Result<()> {
        let value = value.trim();
        if value.is_empty() {
            bail!("missing value for {key}");
        }

        match key {
            "--map" => self.map_path = Some(value.to_owned()),
            "--spawn" => self.spawn_id = Some(value.to_owned()),
            "--scene" => self.scene = Some(parse_scene_id(value)?),
            _ => bail!("unknown launch option: {key}"),
        }

        Ok(())
    }

    fn infer_scene(&mut self) {
        if self.scene.is_none() && self.map_path.is_some() {
            self.scene = Some(SceneId::Overworld);
        }
    }
}

impl AlienArchiveApp {
    fn new(options: GameLaunchOptions) -> Result<Self> {
        let save_path = save_path();
        let save_data = SaveData::load_or_default(&save_path);
        let mut context = GameContext::from_save(save_path, save_data, load_codex_database());
        context.cutscene_database = load_cutscene_database();
        context.event_database = load_event_database();

        let scenes = if let Some(scene_id) = options.scene {
            apply_launch_options_to_context(&mut context, scene_id, options);
            SceneStack::new_scene(scene_id, &context)?
        } else {
            SceneStack::new_main_menu()
        };

        Ok(Self { scenes, context })
    }
}

impl Game for AlienArchiveApp {
    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        self.scenes.setup_current(renderer)
    }

    fn update(&mut self, dt: f32, input: &InputState) -> Result<()> {
        self.scenes.update(&mut self.context, dt, input)?;
        self.context.update_save(dt)
    }

    fn render(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        self.scenes.render(&self.context, renderer)
    }

    fn camera(&self) -> Camera2d {
        self.scenes.camera()
    }

    fn should_exit(&self) -> bool {
        self.context.should_quit
    }
}

fn main() -> Result<()> {
    let launch_options = GameLaunchOptions::from_env_and_args()?;
    run("Alien Archive", AlienArchiveApp::new(launch_options)?)
}

fn load_codex_database() -> CodexDatabase {
    match CodexDatabase::load(std::path::Path::new(DEFAULT_CODEX_DB_PATH)) {
        Ok(database) => database,
        Err(error) => {
            eprintln!("codex database load failed: {error:?}");
            CodexDatabase::new("Overworld")
        }
    }
}

fn load_cutscene_database() -> CutsceneDatabase {
    match CutsceneDatabase::load(std::path::Path::new(DEFAULT_CUTSCENE_DB_PATH)) {
        Ok(database) => database,
        Err(error) => {
            eprintln!("cutscene database load failed: {error:?}");
            CutsceneDatabase::default()
        }
    }
}

fn load_event_database() -> EventDatabase {
    match EventDatabase::load(std::path::Path::new(DEFAULT_EVENT_DB_PATH)) {
        Ok(database) => database,
        Err(error) => {
            eprintln!("event database load failed: {error:?}");
            EventDatabase::default()
        }
    }
}

fn save_path() -> std::path::PathBuf {
    std::env::var("ALIEN_ARCHIVE_SAVE_PATH")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_SAVE_PATH))
}

fn apply_launch_options_to_context(
    context: &mut GameContext,
    scene_id: SceneId,
    options: GameLaunchOptions,
) {
    match scene_id {
        SceneId::Facility => {
            let overrides_position = options.map_path.is_some() || options.spawn_id.is_some();
            if let Some(map_path) = options.map_path {
                context.facility_map_path = Some(map_path);
            }
            if let Some(spawn_id) = options.spawn_id {
                context.facility_spawn_id = Some(spawn_id);
            }
            if overrides_position {
                context.facility_player_position = None;
            }
        }
        _ => {
            let overrides_position = options.map_path.is_some() || options.spawn_id.is_some();
            if let Some(map_path) = options.map_path {
                context.overworld_map_path = Some(map_path);
            }
            if let Some(spawn_id) = options.spawn_id {
                context.overworld_spawn_id = Some(spawn_id);
            }
            if overrides_position {
                context.overworld_player_position = None;
            }
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn required_arg_value(args: &mut impl Iterator<Item = String>, key: &str) -> Result<String> {
    let value = args
        .next()
        .ok_or_else(|| anyhow!("missing value for {key}"))?;
    if value.trim().is_empty() {
        bail!("missing value for {key}");
    }
    Ok(value)
}

fn parse_scene_id(value: &str) -> Result<SceneId> {
    let Some(scene) = semantics::runtime_scene_def(value) else {
        bail!("unknown scene '{value}', expected overworld or facility");
    };

    match scene.key {
        semantics::SCENE_FACILITY => Ok(SceneId::Facility),
        semantics::SCENE_MAIN_MENU => Ok(SceneId::MainMenu),
        _ => Ok(SceneId::Overworld),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_arg_starts_overworld_by_default() {
        let options = GameLaunchOptions::from_args([
            "--map",
            "assets/data/maps/test.ron",
            "--spawn",
            "entry",
        ])
        .expect("launch options should parse");

        assert_eq!(options.scene, Some(SceneId::Overworld));
        assert_eq!(
            options.map_path.as_deref(),
            Some("assets/data/maps/test.ron")
        );
        assert_eq!(options.spawn_id.as_deref(), Some("entry"));
    }

    #[test]
    fn scene_arg_can_target_facility() {
        let options = GameLaunchOptions::from_args([
            "--scene=facility",
            "--map=assets/data/maps/facility_ruin_01.ron",
        ])
        .expect("launch options should parse");

        assert_eq!(options.scene, Some(SceneId::Facility));
        assert_eq!(
            options.map_path.as_deref(),
            Some("assets/data/maps/facility_ruin_01.ron")
        );
    }

    #[test]
    fn unknown_scene_is_rejected() {
        let error = GameLaunchOptions::from_args(["--scene", "space"]).unwrap_err();
        assert!(error.to_string().contains("unknown scene"));
    }
}
