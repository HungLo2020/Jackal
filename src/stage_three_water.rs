//! Stage-three water graphics: DDB8..DDE2 schedules alternating six-part uploads.
use crate::renderer::Texture;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};
#[derive(Clone, Debug)]
pub struct StageThreeWater {
    timer: u8,
    next: u8,
    upload: Option<(u8, u8)>,
    visible: Option<(u8, usize)>,
    pending: Option<(u8, usize)>,
}
impl Default for StageThreeWater {
    fn default() -> Self {
        // The original stage loader executes 25 countdown updates, then six
        // loading frames skip the routine. First playable frame starts at 231.
        Self {
            timer: 231,
            next: 0,
            upload: None,
            visible: None,
            pending: None,
        }
    }
}
impl StageThreeWater {
    pub fn tick(&mut self, other_graphics_busy: bool) {
        self.tick_with_queue(
            other_graphics_busy,
            crate::palette_cycle::GraphicsQueueBudget {
                queued_bytes: 0,
                routine_allowed: true,
            },
        );
    }
    pub fn tick_with_queue(
        &mut self,
        other_graphics_busy: bool,
        queue: crate::palette_cycle::GraphicsQueueBudget,
    ) {
        // The preceding update's queued chunk becomes visible before this
        // update decides whether another chunk fits the shared upload budget.
        if let Some(frame) = self.pending.take() {
            self.visible = Some(frame);
        }
        if other_graphics_busy {
            self.upload = None;
            self.visible = None;
            return;
        }
        if !queue.routine_allowed {
            return;
        }
        if let Some((phase, chunk)) = self.upload {
            if queue.queued_bytes < 24 {
                self.pending = Some((phase, usize::from(chunk)));
                self.upload = if chunk == 5 {
                    None
                } else {
                    Some((phase, chunk + 1))
                };
            }
        } else {
            self.timer = self.timer.wrapping_sub(1);
            if self.timer == 0 {
                self.timer = 16;
                self.upload = Some((self.next, 0));
                self.next ^= 1;
            }
        }
    }
    pub fn frame(&self) -> Option<(u8, usize)> {
        self.visible
    }
}
#[derive(Deserialize)]
struct Manifest {
    initial: String,
    early: [Vec<String>; 2],
    late: [Vec<String>; 2],
}
pub struct StageThreeWaterArt {
    images: HashMap<String, Texture>,
    manifest: Manifest,
}
impl StageThreeWaterArt {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("textures/stage_three_water/animation.json");
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let mut images = HashMap::new();
        for phase in manifest.early.iter().chain(manifest.late.iter()) {
            if phase.len() != 6 {
                return Err("Water uploads require six graphics chunks".into());
            }
            for name in phase {
                if images.contains_key(name) {
                    continue;
                }
                let image = image::open(root.join(name))
                    .map_err(|e| e.to_string())?
                    .to_rgba8();
                if image.dimensions() != (128, 512) {
                    return Err("Invalid water atlas dimensions".into());
                }
                images.insert(
                    name.clone(),
                    Texture::from_rgba(128, 512, image.into_raw())?,
                );
            }
        }
        if !images.contains_key(&manifest.initial) {
            return Err("Missing initial water phase".into());
        }
        Ok(Self { images, manifest })
    }
    pub fn tiles(&self, late: bool, frame: (u8, usize)) -> &Texture {
        let phases = if late {
            &self.manifest.late
        } else {
            &self.manifest.early
        };
        &self.images[&phases[usize::from(frame.0)][frame.1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../tests/fixtures/stage_three_water.json")).unwrap()
    }
    fn registers(water: &StageThreeWater) -> [u8; 3] {
        [
            water.timer,
            water.next,
            water
                .upload
                .filter(|(_, age)| *age < 6)
                .map_or(0, |(phase, _)| phase + 3),
        ]
    }
    #[test]
    fn original_playable_entry_and_eleven_uploads_match_registers_and_every_pixel() {
        let root = crate::assets::Assets::default_directory();
        let art = StageThreeWaterArt::load(&root).unwrap();
        let mut water = StageThreeWater::default();
        let captures = fixture();
        let mut expected = HashMap::new();
        for sample in captures["frames"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["mode"] == 3)
        {
            water.tick(false);
            let after = &sample["after"];
            assert_eq!(
                registers(&water),
                [
                    after[0].as_u64().unwrap() as u8,
                    after[1].as_u64().unwrap() as u8,
                    after[2].as_u64().unwrap() as u8
                ],
                "tick {}",
                sample["tick"]
            );
            let path = sample["tiles"].as_str().unwrap();
            let pixels = expected
                .entry(path.to_owned())
                .or_insert_with(|| image::open(root.join(path)).unwrap().to_rgba8().into_raw());
            let actual = water
                .frame()
                .map_or(&art.images[&art.manifest.initial], |frame| {
                    art.tiles(false, frame)
                });
            assert_eq!(
                actual.rgba(),
                pixels.as_slice(),
                "water upload pixels tick {}",
                sample["tick"]
            );
        }
    }
    #[test]
    fn palette_upload_interrupts_water_and_preserves_countdown_and_next_phase() {
        let captures = fixture();
        let first = &captures["palette_transition"][0]["before"];
        let mut water = StageThreeWater {
            timer: first[0].as_u64().unwrap() as u8,
            next: first[1].as_u64().unwrap() as u8,
            upload: Some((1, 2)),
            visible: Some((1, 0)),
            pending: None,
        };
        for sample in captures["palette_transition"].as_array().unwrap() {
            let external = sample["before"][2] == 1;
            water.tick(external);
            let after = &sample["after"];
            assert_eq!(
                [water.timer, water.next],
                [
                    after[0].as_u64().unwrap() as u8,
                    after[1].as_u64().unwrap() as u8
                ]
            );
            if external {
                assert!(water.frame().is_none());
                assert!(water.upload.is_none());
            } else {
                assert_eq!(registers(&water)[2], after[2].as_u64().unwrap() as u8);
            }
        }
    }
    #[test]
    fn game_loading_pause_and_other_stages_do_not_advance_water() {
        use crate::{
            game::{Game, GameState},
            input::Input,
        };
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let mut game = Game::new();
        game.start_stage(2, &assets);
        game.state = GameState::StageLoading { ticks: 0 };
        for _ in 0..10 {
            game.update(Input::default(), &assets);
        }
        assert_eq!(game.stage_three_water.timer, 231);
        game.state = GameState::Playing;
        game.paused = true;
        game.update(Input::default(), &assets);
        assert_eq!(game.stage_three_water.timer, 231);
        game.paused = false;
        game.update(Input::default(), &assets);
        assert_eq!(game.stage_three_water.timer, 230);
        game.start_stage(1, &assets);
        game.state = GameState::Playing;
        game.update(Input::default(), &assets);
        assert_eq!(game.stage_three_water.timer, 231);
    }
    #[test]
    fn scrolling_map_uploads_and_palette_cycles_match_original_water_pixels() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/water_moving.json")).unwrap();
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let art = &assets.stage_three_water_art;
        let mut water = StageThreeWater::default();
        let mut palette = crate::palette_cycle::PaletteCycle::default();
        let mut cache = HashMap::new();
        for sample in captures.as_array().unwrap() {
            let queue = palette.update(2, sample["distance"].as_u64().unwrap() as u32, 0);
            water.tick_with_queue(false, queue);
            let expected = &sample["water"];
            assert_eq!(
                registers(&water),
                [
                    expected[0].as_u64().unwrap() as u8,
                    expected[1].as_u64().unwrap() as u8,
                    expected[2].as_u64().unwrap() as u8
                ],
                "moving tick {}",
                sample["tick"]
            );
            let (signature, pixels, indices) = cache
                .entry((water.frame(), palette.visible()))
                .or_insert_with(|| {
                    let source = water
                        .frame()
                        .map_or(&art.images[&art.manifest.initial], |f| art.tiles(false, f));
                    let composed = palette
                        .visible()
                        .map(|p| assets.palette_cycle_art.compose(source, 2, p));
                    let colors = composed.as_ref().unwrap_or(source);
                    let mut hash = 0xcbf29ce484222325u64;
                    let mut pixels = vec![];
                    let mut indices = vec![];
                    for tile in 222..234 {
                        for y in 0..8 {
                            for x in 0..8 {
                                let offset = ((tile / 16 * 8 + y) * 128 + tile % 16 * 8 + x) * 4;
                                let p = &source.rgba()[offset..offset + 3];
                                let rgb = u32::from_be_bytes([0, p[0], p[1], p[2]]);
                                let index = [0, 0x003c76, 0x0d57ff, 0xfa9e00]
                                    .iter()
                                    .position(|&c| c == rgb)
                                    .unwrap();
                                hash ^= index as u64;
                                hash = hash.wrapping_mul(0x100000001b3);
                                let p = &colors.rgba()[offset..offset + 3];
                                pixels.push(u32::from_be_bytes([0, p[0], p[1], p[2]]));
                                indices.push(index);
                            }
                        }
                    }
                    (hash, pixels, indices)
                });
            assert_eq!(
                *signature,
                sample["signature"].as_u64().unwrap(),
                "moving water pattern tick {}",
                sample["tick"]
            );
            for (&pixel, &index) in pixels.iter().zip(indices.iter()) {
                assert_eq!(
                    pixel,
                    sample["palette"][index].as_u64().unwrap() as u32,
                    "moving water color tick {}",
                    sample["tick"]
                );
            }
        }
    }
}
