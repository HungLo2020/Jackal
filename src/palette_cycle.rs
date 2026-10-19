//! Timed background palette animation recovered from DDB0..DE26.
use crate::renderer::Texture;
use serde::Deserialize;
use std::path::Path;
const PAGE_RANGES: [[u8; 2]; 6] = [[9, 11], [7, 9], [8, 10], [9, 11], [9, 11], [4, 6]];
/// Deferred native graphics work for the current update.
#[derive(Clone, Copy, Debug)]
pub struct GraphicsQueueBudget {
    pub queued_bytes: usize,
    pub routine_allowed: bool,
}
#[derive(Clone, Debug)]
pub struct PaletteCycle {
    timer: u8,
    next: u8,
    pending: Option<u8>,
    visible: Option<u8>,
    previous_distance: u32,
    row_upload: bool,
    freeze_delay: u8,
}
impl Default for PaletteCycle {
    fn default() -> Self {
        Self {
            timer: 7,
            next: 0,
            pending: None,
            visible: None,
            previous_distance: 240,
            row_upload: false,
            freeze_delay: 0,
        }
    }
}
impl PaletteCycle {
    /// Queue length is the number of bytes already scheduled for the next
    /// graphics transfer. A full queue postpones a request, not its countdown.
    pub fn tick(&mut self, stage: usize, page: u8, queued_bytes: usize) {
        if let Some(phase) = self.pending.take() {
            self.visible = Some(phase);
        }
        if self.timer == 0 {
            if queued_bytes >= 33 {
                return;
            }
            self.timer = 32;
            let [start, end] = PAGE_RANGES[stage];
            if (start..end).contains(&page) {
                self.pending = Some(self.next);
                self.next = (self.next + 1) & 3;
            }
        } else if self.timer < 128 {
            self.timer -= 1;
        }
    }
    pub fn update(
        &mut self,
        stage: usize,
        distance: u32,
        additional_queued_bytes: usize,
    ) -> GraphicsQueueBudget {
        let queued = additional_queued_bytes + if self.row_upload { 72 } else { 0 };
        self.row_upload = distance / 8 != self.previous_distance / 8;
        self.previous_distance = distance;
        if self.freeze_delay != 0 {
            self.freeze_delay -= 1;
            if self.freeze_delay == 0 {
                self.timer = 255;
            }
        }
        let routine_allowed = self.timer != 0 || queued < 33;
        self.tick(stage, (distance / 240) as u8, queued);
        GraphicsQueueBudget {
            queued_bytes: queued + if self.pending.is_some() { 20 } else { 0 },
            routine_allowed,
        }
    }
    /// F1 schedules its preparation after the current update's rendering work.
    pub fn prepare_palette(&mut self) {
        self.freeze_delay = 2;
    }
    pub fn visible(&self) -> Option<u8> {
        self.visible
    }
}
#[derive(Deserialize)]
struct Entry {
    page_range: [u8; 2],
    swatches: String,
}
pub struct PaletteCycleArt {
    colors: Vec<[[u32; 32]; 5]>,
}
impl PaletteCycleArt {
    pub fn load(root: &Path) -> Result<Self, String> {
        let entries: Vec<Entry> = serde_json::from_slice(
            &std::fs::read(root.join("textures/palette_cycles/animation.json"))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if entries.len() != 6 {
            return Err("Expected six palette-cycle definitions".into());
        }
        let mut colors = vec![];
        for (stage, e) in entries.into_iter().enumerate() {
            if e.page_range != PAGE_RANGES[stage] {
                return Err("Invalid palette-cycle page range".into());
            }
            let image = image::open(root.join(e.swatches))
                .map_err(|e| e.to_string())?
                .to_rgb8();
            if image.dimensions() != (32, 5) {
                return Err("Invalid palette-cycle swatch dimensions".into());
            }
            let mut swatches = [[0; 32]; 5];
            for y in 0..5 {
                for x in 0..32 {
                    let p = image.get_pixel(x, y).0;
                    swatches[y as usize][x as usize] = u32::from_be_bytes([0, p[0], p[1], p[2]]);
                }
            }
            for band in 0..4 {
                for a in 0..4 {
                    for b in a + 1..4 {
                        if swatches[0][band * 4 + a] == swatches[0][band * 4 + b] {
                            return Err("Ambiguous background palette color indices".into());
                        }
                    }
                }
            }
            for phase in &swatches[1..] {
                if phase[16..] != swatches[0][16..] || phase[0] != swatches[0][0] {
                    return Err("Background cycle alters sprite or universal colors".into());
                }
            }
            colors.push(swatches);
        }
        Ok(Self { colors })
    }
    /// Recolor by palette band, retaining the selected native tile animation.
    /// All four source colors in each shipped band are distinct; validation
    /// prevents ambiguous RGB-to-index reconstruction.
    pub fn compose(&self, source: &Texture, stage: usize, phase: u8) -> Texture {
        assert_eq!((source.width, source.height), (128, 512));
        let colors = &self.colors[stage];
        let mut pixels = source.rgba().to_vec();
        for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let band = (i / source.width) / 128;
            let rgb = u32::from_be_bytes([0, pixel[0], pixel[1], pixel[2]]);
            if let Some(index) = colors[0][band * 4..band * 4 + 4]
                .iter()
                .position(|&c| c == rgb)
            {
                let mapped = colors[usize::from(phase) + 1][band * 4 + index].to_be_bytes();
                pixel[..3].copy_from_slice(&mapped[1..]);
            } else {
                panic!("Background atlas uses a color absent from its palette band");
            }
        }
        Texture::from_rgba(128, 512, pixels).expect("preserved texture dimensions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_stage_scroll_matches_original_timer_requests_and_delivered_colors() {
        let traces: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/palette_cycles.json")).unwrap();
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        for trace in traces.as_array().unwrap() {
            let stage = trace["stage"].as_u64().unwrap() as usize;
            let mut cycle = PaletteCycle::default();
            let mut prepared = false;
            for frame in trace["frames"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["mode"] == 3)
            {
                let distance = frame["distance"].as_u64().unwrap() as u32;
                if !prepared
                    && assets.levels[stage]
                        .commands
                        .iter()
                        .any(|c| c.command == "palette_prepare" && c.distance <= distance as f32)
                {
                    cycle.prepare_palette();
                    prepared = true;
                }
                cycle.update(stage, distance, 0);
                assert_eq!(
                    [cycle.timer, cycle.next],
                    [
                        frame["after"][0].as_u64().unwrap() as u8,
                        frame["after"][1].as_u64().unwrap() as u8
                    ],
                    "stage {stage} tick {}",
                    frame["tick"]
                );
                // Palette-prepare/change commands subsequently own these colors.
                let first_command = assets.levels[stage]
                    .commands
                    .iter()
                    .find(|c| c.command != "forced_scroll")
                    .unwrap()
                    .distance;
                if (distance as f32) < first_command {
                    let colors = &assets.palette_cycle_art.colors[stage]
                        [cycle.visible.map_or(0, |p| usize::from(p) + 1)];
                    for (i, &color) in colors.iter().enumerate() {
                        assert_eq!(
                            color,
                            frame["palette"][i].as_u64().unwrap() as u32,
                            "stage {stage} tick {} palette {i}",
                            frame["tick"]
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn preparation_freezes_on_the_next_update_even_when_scrolling_stops() {
        let mut cycle = PaletteCycle::default();
        cycle.prepare_palette();
        cycle.update(0, 2674, 0);
        assert_eq!(cycle.timer, 6);
        cycle.update(0, 2674, 0);
        assert_eq!(cycle.timer, 255);
        for _ in 0..100 {
            cycle.update(0, 2674, 0);
        }
        assert_eq!(cycle.timer, 255);
    }
    #[test]
    fn queue_gates_only_requests_and_delivery_precedes_next_countdown() {
        let mut p = PaletteCycle {
            timer: 0,
            ..Default::default()
        };
        p.tick(0, 9, 33);
        assert_eq!(p.timer, 0);
        assert_eq!(p.pending, None);
        p.tick(0, 9, 32);
        assert_eq!(p.timer, 32);
        assert_eq!(p.visible, None);
        p.tick(0, 9, 255);
        assert_eq!(p.timer, 31);
        assert_eq!(p.visible, Some(0));
        p.timer = 128;
        p.tick(0, 9, 0);
        assert_eq!(p.timer, 128);
    }
    #[test]
    fn every_early_river_and_initial_atlas_accepts_lossless_palette_composition() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        for stage in 0..6 {
            for phase in 0..4 {
                assets
                    .palette_cycle_art
                    .compose(assets.textures(stage, 0., 0).0, stage, phase);
            }
        }
        for age in 0..40 {
            for phase in 0..4 {
                assets
                    .palette_cycle_art
                    .compose(assets.river_art.textures(false, age).0, 5, phase);
            }
        }
    }
    #[test]
    fn cycles_compose_with_every_early_water_frame_without_changing_other_bands() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        for water_phase in 0..2 {
            for chunk in 0..6 {
                for phase in 0..4 {
                    let source = assets
                        .stage_three_water_art
                        .tiles(false, (water_phase, chunk));
                    let result = assets.palette_cycle_art.compose(source, 2, phase);
                    assert_eq!(
                        &result.rgba()[128 * 128 * 4..],
                        &source.rgba()[128 * 128 * 4..]
                    );
                    for (input, output) in source
                        .rgba()
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(result.rgba().as_chunks::<4>().0.iter())
                    {
                        if input[..3] == [0, 0, 0] {
                            assert_eq!(input, output);
                        }
                    }
                }
            }
        }
    }
}
