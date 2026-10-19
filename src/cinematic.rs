//! Recorded noninteractive presentation, expressed as PNG artwork and JSON timelines.
//!
//! Each frame is an ordinary 32 by 30 tile composition. Repeated tiles share a PNG
//! atlas and consecutive identical compositions share a duration. Playback uses
//! native drawing only; there is no machine state or executable data in a clip.
use crate::renderer::{Canvas, Rect, Texture};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
struct Timeline {
    atlas: String,
    tile_size: usize,
    atlas_columns: usize,
    duration_ticks: u32,
    frames: Vec<Frame>,
    #[serde(default)]
    sound_commands: Vec<[u32; 2]>,
}
#[derive(Deserialize)]
struct Frame {
    duration_ticks: u32,
    tiles: Vec<[u32; 2]>,
}
struct Composition {
    end_tick: u32,
    tiles: Vec<u32>,
}
pub struct Cinematic {
    atlas: Texture,
    columns: usize,
    compositions: Vec<Composition>,
    duration: u32,
    sound_commands: Vec<[u32; 2]>,
}
impl Cinematic {
    fn load(directory: &Path) -> Result<Self, String> {
        let path = directory.join("timeline.json");
        let timeline: Timeline = serde_json::from_slice(
            &std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        if timeline.tile_size != 8 || timeline.atlas_columns == 0 || timeline.frames.is_empty() {
            return Err(format!(
                "{}: expected 8-pixel tiles and a nonempty timeline",
                path.display()
            ));
        }
        if Path::new(&timeline.atlas).components().count() != 1 {
            return Err(format!(
                "{}: atlas must be a local filename",
                path.display()
            ));
        }
        let image = image::open(directory.join(&timeline.atlas))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let (width, height) = image.dimensions();
        if width as usize != timeline.atlas_columns * 8 || height == 0 || height % 8 != 0 {
            return Err(format!(
                "{}: atlas dimensions do not match tile layout",
                path.display()
            ));
        }
        let tile_count = (width / 8) * (height / 8);
        let mut elapsed = 0u32;
        let mut compositions = Vec::with_capacity(timeline.frames.len());
        for frame in timeline.frames {
            if frame.duration_ticks == 0 {
                return Err("cinematic frame has zero duration".into());
            }
            elapsed = elapsed
                .checked_add(frame.duration_ticks)
                .ok_or("cinematic duration overflow")?;
            let mut tiles = Vec::with_capacity(960);
            for [count, tile] in frame.tiles {
                if count == 0
                    || count > 960
                    || tile >= tile_count
                    || tiles.len() + count as usize > 960
                {
                    return Err("cinematic tile run exceeds frame or atlas bounds".into());
                }
                tiles.extend(std::iter::repeat_n(tile, count as usize));
            }
            if tiles.len() != 960 {
                return Err("cinematic frame must cover 256 by 240 pixels".into());
            }
            compositions.push(Composition {
                end_tick: elapsed,
                tiles,
            });
        }
        if elapsed != timeline.duration_ticks {
            return Err("cinematic duration does not match its frames".into());
        }
        if timeline
            .sound_commands
            .iter()
            .any(|[tick, command]| *tick >= elapsed || *command > 255)
            || timeline
                .sound_commands
                .windows(2)
                .any(|pair| pair[0][0] > pair[1][0])
        {
            return Err("cinematic sound commands are outside the timeline or unordered".into());
        }
        Ok(Self {
            sound_commands: timeline.sound_commands,
            atlas: Texture::from_rgba(width as usize, height as usize, image.into_raw())?,
            columns: timeline.atlas_columns,
            compositions,
            duration: elapsed,
        })
    }
    pub fn duration_ticks(&self) -> u32 {
        self.duration
    }
    /// Recovered sound-driver commands issued at this exact presentation tick.
    pub fn sound_commands_at(&self, tick: u32) -> impl Iterator<Item = u8> + '_ {
        self.sound_commands
            .iter()
            .filter(move |event| event[0] == tick)
            .map(|event| event[1] as u8)
    }
    /// Draw a frame; ticks beyond the end hold the final composition.
    pub fn draw(&self, canvas: &mut Canvas, tick: u32) {
        let index = self
            .compositions
            .partition_point(|frame| frame.end_tick <= tick.min(self.duration - 1));
        for (cell, &tile) in self.compositions[index].tiles.iter().enumerate() {
            let tile = tile as usize;
            canvas.draw_region(
                &self.atlas,
                Rect::new(
                    ((tile % self.columns) * 8) as i32,
                    ((tile / self.columns) * 8) as i32,
                    8,
                    8,
                ),
                Rect::new(((cell % 32) * 8) as i32, ((cell / 32) * 8) as i32, 8, 8),
                false,
                false,
            );
        }
    }
}
#[derive(Default)]
pub struct Cinematics {
    clips: BTreeMap<String, Cinematic>,
}
impl Cinematics {
    /// `root` is the asset directory containing `cinematics/manifest.json`.
    pub fn load(root: &Path) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct AudioManifest {
            sounds: Vec<AudioCommand>,
        }
        #[derive(Deserialize)]
        struct AudioCommand {
            command: u8,
        }
        let audio: AudioManifest = serde_json::from_slice(
            &std::fs::read(root.join("audio.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let available_commands: std::collections::BTreeSet<u8> = audio
            .sounds
            .iter()
            .map(|sound| sound.command)
            .chain([0, 0x59])
            .collect();
        let directory = root.join("cinematics");
        let names: Vec<String> = serde_json::from_slice(
            &std::fs::read(directory.join("manifest.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let mut clips = BTreeMap::new();
        for name in names {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err("invalid cinematic name".into());
            }
            let clip = Cinematic::load(&directory.join(&name))?;
            if clip
                .sound_commands
                .iter()
                .any(|event| !available_commands.contains(&(event[1] as u8)))
            {
                return Err(format!(
                    "cinematic {name} uses a sound command absent from audio.json"
                ));
            }
            if clips.insert(name, clip).is_some() {
                return Err("duplicate cinematic name".into());
            }
        }
        Ok(Self { clips })
    }
    pub fn get(&self, name: &str) -> Option<&Cinematic> {
        self.clips.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_timelines_are_rejected_before_playback() {
        let directory = std::env::temp_dir().join(format!(
            "jackal-cinematic-validation-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        image::RgbaImage::new(8, 8)
            .save(directory.join("tiles.png"))
            .unwrap();
        let valid = serde_json::json!({"atlas":"tiles.png", "tile_size":8, "atlas_columns":1, "duration_ticks":1, "frames":[{"duration_ticks":1,"tiles":[[960,0]]}]});
        let mut cases = Vec::new();
        let mut value = valid.clone();
        value["frames"][0]["tiles"][0][0] = 961.into();
        cases.push(value);
        let mut value = valid.clone();
        value["frames"][0]["tiles"][0][1] = 1.into();
        cases.push(value);
        let mut value = valid.clone();
        value["frames"][0]["duration_ticks"] = 0.into();
        cases.push(value);
        let mut value = valid.clone();
        value["duration_ticks"] = 2.into();
        cases.push(value);
        let mut value = valid.clone();
        value["sound_commands"] = serde_json::json!([[1, 52]]);
        cases.push(value);
        for value in cases {
            std::fs::write(
                directory.join("timeline.json"),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
            assert!(Cinematic::load(&directory).is_err());
        }
        std::fs::write(
            directory.join("timeline.json"),
            serde_json::to_vec(&valid).unwrap(),
        )
        .unwrap();
        assert!(Cinematic::load(&directory).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
