//! Standard, editable assets. Loading validates references before gameplay starts.
use crate::{math::Vec2, renderer::Texture};
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Spawn {
    pub distance: f32,
    pub x: f32,
    pub kind: String,
    pub source_type: u8,
    #[serde(default)]
    pub priority_spawn: bool,
    #[serde(default)]
    pub from_bottom: bool,
    #[serde(default)]
    pub odd_difficulty_only: bool,
}
#[derive(Clone, Debug, Deserialize)]
pub struct LevelCommand {
    pub distance: f32,
    pub command: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct BossDefinition {
    pub kind: String,
    pub distance: f32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Level {
    pub name: String,
    pub width_tiles: usize,
    pub height_tiles: usize,
    pub tile_size: usize,
    pub tiles: Vec<u16>,
    pub palettes: Vec<u8>,
    pub collision: Vec<u8>,
    pub events: Vec<Spawn>,
    pub commands: Vec<LevelCommand>,
    pub boss: BossDefinition,
    pub initial_camera_from_bottom: usize,
    pub start_camera_x: f32,
}
impl Level {
    pub fn width(&self) -> f32 {
        (self.width_tiles * self.tile_size) as f32
    }
    pub fn height(&self) -> f32 {
        (self.height_tiles * self.tile_size) as f32
    }
    pub fn start_camera_y(&self) -> f32 {
        self.height() - self.initial_camera_from_bottom as f32
    }
    pub fn terrain_at(&self, point: Vec2) -> u8 {
        if point.x < 0.0 || point.y < 0.0 || point.x >= self.width() || point.y >= self.height() {
            return 1;
        }
        let x = point.x as usize / self.tile_size;
        let y = point.y as usize / self.tile_size;
        self.collision[y * self.width_tiles + x]
    }
    pub fn traversable(&self, point: Vec2, marsh: bool) -> bool {
        let terrain = self.terrain_at(point);
        terrain == 0 || (marsh && terrain == 3)
    }
    pub fn clear_for_jeep(&self, point: Vec2, marsh: bool) -> bool {
        [
            Vec2::new(-4.0, -6.0),
            Vec2::new(4.0, -6.0),
            Vec2::new(-4.0, 6.0),
            Vec2::new(4.0, 6.0),
        ]
        .iter()
        .all(|offset| self.traversable(point + *offset, marsh))
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct SpritePart {
    pub x: i32,
    pub y: i32,
    pub tile: u16,
    pub palette: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub behind_background: bool,
}
#[derive(Clone, Debug, Deserialize)]
pub struct SpriteFrame {
    pub id: u16,
    pub parts: Vec<SpritePart>,
}
#[derive(Deserialize)]
struct SpriteFile {
    frames: Vec<SpriteFrame>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct EntityDefinition {
    pub source_type: u8,
    pub kind: String,
    pub family: String,
    pub hit_points: u16,
    pub grenade_only: bool,
    #[serde(default)]
    pub secondary_passes_through: bool,
    pub half_width: f32,
    pub half_height: f32,
    pub score: u32,
    pub animation: Animation,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Animation {
    pub frames: Vec<u16>,
    pub frame_ticks: u16,
}
#[derive(Deserialize)]
struct EntityFile {
    specs: Vec<EntityDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DestructionDefinition {
    pub stage: usize,
    pub source_type: u8,
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<u16>,
    pub collision: u8,
}
#[derive(Deserialize)]
struct DestructionFile {
    patches: Vec<DestructionDefinition>,
}

pub struct Assets {
    pub texture_phases: Vec<Vec<crate::world_art::TexturePhase>>,
    pub boss_patches: Vec<crate::world_art::BossTilePatch>,
    pub hud: crate::hud::Hud,
    pub river_art: crate::river_art::RiverArt,
    pub palette_cycle_art: crate::palette_cycle::PaletteCycleArt,
    pub stage_three_water_art: crate::stage_three_water::StageThreeWaterArt,
    pub headquarters_art: crate::hq_effects::CollapseArt,
    pub scoreboard: crate::victory_scene::VictoryScoreboard,
    pub cinematics: crate::cinematic::Cinematics,
    pub levels: Vec<Level>,
    pub tiles: Vec<Texture>,
    pub sprites: Vec<Texture>,
    pub frames: Vec<SpriteFrame>,
    pub animations: std::collections::BTreeMap<String, Animation>,
    pub entities: Vec<EntityDefinition>,
    pub destruction: Vec<DestructionDefinition>,
    pub title: Texture,
}
impl Assets {
    pub fn default_directory() -> PathBuf {
        asset_directory(
            std::env::var_os("BASALT_APP_DIR")
                .map(PathBuf::from)
                .as_deref(),
            std::env::var_os("BASALT_GAME_DIR")
                .map(PathBuf::from)
                .as_deref(),
            std::env::current_exe().ok().as_deref(),
        )
    }
    pub fn load(root: &Path) -> Result<Self, Box<dyn Error>> {
        let mut levels = Vec::new();
        let mut tiles = Vec::new();
        let mut sprites = Vec::new();
        for stage in 1..=6 {
            let level: Level = read_json(&root.join(format!("levels/stage_{stage:02}.json")))?;
            let n = level
                .width_tiles
                .checked_mul(level.height_tiles)
                .ok_or("level dimensions overflow")?;
            if level.tile_size != 8
                || level.width_tiles < 32
                || level.height_tiles < 32
                || level.initial_camera_from_bottom < 240
                || level.initial_camera_from_bottom > level.height_tiles * 8
                || !(0.0..=level.width() - 256.0).contains(&level.start_camera_x)
            {
                return Err(format!("Stage {stage}: invalid dimensions or initial camera").into());
            }
            if n == 0
                || level.tiles.len() != n
                || level.palettes.len() != n
                || level.collision.len() != n
            {
                return Err(format!("Stage {stage}: inconsistent tile layer lengths").into());
            }
            if level.tiles.iter().any(|&t| t > 255)
                || level.palettes.iter().any(|&p| p > 3)
                || level.collision.iter().any(|&c| c > 3)
            {
                return Err(format!("Stage {stage}: invalid tile/palette/collision index").into());
            }
            if level
                .events
                .windows(2)
                .any(|e| e[0].distance > e[1].distance)
            {
                return Err(format!("Stage {stage}: events must be ordered by distance").into());
            }
            levels.push(level);
            tiles.push(load_png(
                &root.join(format!("textures/stage_{stage:02}_tiles.png")),
            )?);
            sprites.push(load_png(
                &root.join(format!("textures/stage_{stage:02}_sprites.png")),
            )?);
        }
        for (stage, (tiles, sprites)) in tiles.iter().zip(&sprites).enumerate() {
            if (tiles.width, tiles.height) != (128, 512)
                || (sprites.width, sprites.height) != (128, 1024)
            {
                return Err(format!("Stage {}: invalid atlas dimensions", stage + 1).into());
            }
        }
        let frames = read_json::<SpriteFile>(&root.join("levels/sprites.json"))?.frames;
        if frames.len() != 256
            || frames.iter().enumerate().any(|(i, f)| {
                f.id as usize != i || f.parts.iter().any(|p| p.tile > 255 || p.palette > 3)
            })
        {
            return Err("Invalid sprite atlas references".into());
        }
        let entities = read_json::<EntityFile>(&root.join("levels/entity_specs.json"))?.specs;
        let mut ids = std::collections::HashSet::new();
        for entity in &entities {
            if !ids.insert(entity.source_type) {
                return Err("Duplicate entity ID".into());
            }
            if ![
                "infantry",
                "turret",
                "vehicle",
                "boat",
                "prison",
                "destructible",
                "helicopter",
                "environment",
                "rescue_zone",
                "pickup",
                "prisoner",
                "boss",
                "effect",
            ]
            .contains(&entity.family.as_str())
            {
                return Err(format!("Unknown entity family {}", entity.family).into());
            }
            if entity.animation.frame_ticks == 0
                || entity
                    .animation
                    .frames
                    .iter()
                    .any(|&id| id as usize >= frames.len())
            {
                return Err(format!("Invalid entity animation {}", entity.kind).into());
            }
        }
        for level in &levels {
            for event in &level.events {
                if !entities.iter().any(|e| e.source_type == event.source_type) {
                    return Err(format!(
                        "{} references undefined entity {}",
                        level.name, event.source_type
                    )
                    .into());
                }
            }
        }
        let animations: std::collections::BTreeMap<String, Animation> =
            read_json(&root.join("animations.json"))?;
        for (name, clip) in &animations {
            if clip.frames.is_empty()
                || clip.frame_ticks == 0
                || clip.frames.iter().any(|&id| id as usize >= frames.len())
            {
                return Err(format!("Invalid animation {name}").into());
            }
        }
        for name in [
            "jeep_north",
            "jeep_northeast",
            "jeep_east",
            "prisoner",
            "pickup",
            "gun_bullet",
            "grenade",
            "infantry_death",
            "jeep_death",
        ] {
            if !animations.contains_key(name) {
                return Err(format!("Missing animation {name}").into());
            }
        }
        let destruction = read_json::<DestructionFile>(&root.join("destruction.json"))?.patches;
        let mut destruction_ids = std::collections::HashSet::new();
        for patch in &destruction {
            if !(1..=levels.len()).contains(&patch.stage)
                || !destruction_ids.insert((patch.stage, patch.source_type))
                || patch.width == 0
                || patch.height == 0
                || patch.width.checked_mul(patch.height) != Some(patch.tiles.len())
                || patch.collision > 3
                || patch.tiles.iter().any(|&tile| tile > 255)
                || !entities
                    .iter()
                    .any(|entity| entity.source_type == patch.source_type)
            {
                return Err("Invalid destruction tile definition".into());
            }
        }
        for (stage, level) in levels.iter().enumerate() {
            for event in &level.events {
                if entities
                    .iter()
                    .any(|e| e.source_type == event.source_type && e.family == "prison")
                    && !destruction_ids.contains(&(stage + 1, event.source_type))
                {
                    return Err(
                        format!("Stage {}: missing prison destruction tiles", stage + 1).into(),
                    );
                }
            }
        }
        let (texture_phases, boss_patches) = crate::world_art::load(root)?;
        Ok(Self {
            texture_phases,
            boss_patches,
            hud: crate::hud::Hud::load(root)?,
            river_art: crate::river_art::RiverArt::load(root)?,
            palette_cycle_art: crate::palette_cycle::PaletteCycleArt::load(root)?,
            stage_three_water_art: crate::stage_three_water::StageThreeWaterArt::load(root)?,
            headquarters_art: crate::hq_effects::CollapseArt::load(root)?,
            cinematics: crate::cinematic::Cinematics::load(root)?,
            scoreboard: crate::victory_scene::VictoryScoreboard::load(root)?,
            destruction,
            animations,
            levels,
            tiles,
            sprites,
            frames,
            entities,
            title: load_png(&root.join("textures/title.png"))?,
        })
    }
    pub fn textures(&self, stage: usize, distance: f32, age: u64) -> (&Texture, &Texture) {
        let active_distance = self.texture_phases[stage]
            .iter()
            .rev()
            .find(|phase| phase.distance <= distance)
            .map(|phase| phase.distance)
            .unwrap_or(-1.);
        match self.texture_phases[stage].iter().rev().find(|phase| {
            phase.distance <= distance
                && (phase.distance < active_distance || phase.tick_offset <= age)
        }) {
            Some(phase) => (&phase.tiles, &phase.sprites),
            None => (&self.tiles[stage], &self.sprites[stage]),
        }
    }
    pub fn animation_frame(&self, name: &str, tick: u64) -> u16 {
        let clip = &self.animations[name];
        clip.frames[(tick / clip.frame_ticks as u64) as usize % clip.frames.len()]
    }
    pub fn entity(&self, id: u8) -> &EntityDefinition {
        self.entities
            .iter()
            .find(|e| e.source_type == id)
            .expect("spawn definitions validated on load")
    }
    pub fn destruction(&self, stage: usize, source_type: u8) -> Option<&DestructionDefinition> {
        self.destruction
            .iter()
            .find(|patch| patch.stage == stage + 1 && patch.source_type == source_type)
    }
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&data).map_err(|e| format!("{}: {e}", path.display()).into())
}
/// An installed executable must never fall back to a build-machine checkout,
/// even when its packaged assets are missing. Cargo builds retain source assets.
fn asset_directory(app: Option<&Path>, install: Option<&Path>, exe: Option<&Path>) -> PathBuf {
    if let Some(app) = app.filter(|p| !p.as_os_str().is_empty()) {
        return app.join("assets");
    }
    if let Some(install) = install.filter(|p| !p.as_os_str().is_empty()) {
        return install.join("app/assets");
    }
    if let Some(bin) = exe.and_then(Path::parent) {
        if bin.file_name() == Some(std::ffi::OsStr::new("bin")) {
            if let Some(app) = bin.parent() {
                return app.join("assets");
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn load_png(path: &Path) -> Result<Texture, Box<dyn Error>> {
    let image = image::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .into_rgba8();
    Texture::from_rgba(
        image.width() as usize,
        image.height() as usize,
        image.into_raw(),
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn installed_assets_follow_basalt_or_the_executable_without_checkout_fallback() {
        let exe = Path::new("/a moved library/Jackal/app/bin/jackal-native");
        assert_eq!(
            asset_directory(None, None, Some(exe)),
            Path::new("/a moved library/Jackal/app/assets")
        );
        assert_eq!(
            asset_directory(None, Some(Path::new("/library/Jackal")), Some(exe)),
            Path::new("/library/Jackal/app/assets")
        );
        assert_eq!(
            asset_directory(
                Some(Path::new("/custom/app")),
                Some(Path::new("/library/Jackal")),
                Some(exe)
            ),
            Path::new("/custom/app/assets")
        );
        assert_eq!(
            asset_directory(Some(Path::new("")), None, Some(exe)),
            Path::new("/a moved library/Jackal/app/assets")
        );
    }

    #[test]
    fn cargo_runs_and_tests_keep_using_source_assets() {
        for executable in [
            "/checkout/target/debug/jackal-native",
            "/checkout/target/release/jackal-native",
            "/checkout/target/debug/deps/test-binary",
        ] {
            assert_eq!(
                asset_directory(None, None, Some(Path::new(executable))),
                Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
            );
        }
    }
}
