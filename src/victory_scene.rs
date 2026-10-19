//! Dynamic victory counters drawn with the original eight-pixel digit artwork.
//!
//! The recovered cinematic supplies scenery and labels. Counters remain live
//! game data, so recordings never substitute captured scores for player progress.
use crate::renderer::{Canvas, Rect, Texture};
use std::path::Path;

pub struct VictoryScoreboard {
    digits: Texture,
}
impl VictoryScoreboard {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let path = assets.join("cinematics/score_digits.png");
        let image = image::open(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .into_rgba8();
        if image.dimensions() != (80, 8) {
            return Err("victory digit atlas must contain ten 8x8 digits".into());
        }
        Ok(Self {
            digits: Texture::from_rgba(80, 8, image.into_raw())?,
        })
    }
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        scores: &[u32; 2],
        delivered: &[u32; 2],
        selected_players: u8,
        scene_tick: u32,
    ) {
        // Original scoreboard finishes its blank-screen transition on tick143.
        if scene_tick < 143 {
            return;
        }
        for player in 0..2 {
            let enabled = player < usize::from(selected_players);
            let score = if enabled { scores[player] } else { 0 };
            let rescued = if enabled { delivered[player] } else { 0 };
            let y = 64 + player as i32 * 72;
            self.number(canvas, score.min(999_999), 6, 176, y);
            self.number(canvas, rescued.min(99), 2, 152, y + 24);
        }
    }
    fn number(&self, canvas: &mut Canvas, number: u32, digits: usize, x: i32, y: i32) {
        let text = format!("{number:0digits$}");
        for (index, digit) in text.bytes().enumerate() {
            canvas.draw_region(
                &self.digits,
                Rect::new(i32::from(digit - b'0') * 8, 0, 8, 8),
                Rect::new(x + index as i32 * 8, y, 8, 8),
                false,
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoreboard_uses_live_digits_and_preserves_transition() {
        let board =
            VictoryScoreboard::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
        let mut canvas = Canvas::default();
        canvas.clear(0x123456);
        board.draw(&mut canvas, &[123456, 789012], &[12, 34], 2, 142);
        assert!(canvas.pixels().iter().all(|&pixel| pixel == 0x123456));
        board.draw(&mut canvas, &[123456, 789012], &[12, 34], 2, 143);
        for (x, y, digit) in [(176, 64, 1), (216, 64, 6), (176, 136, 7), (152, 160, 3)] {
            for row in 0..8 {
                for column in 0..8 {
                    let index = (row * 80 + digit * 8 + column) * 4;
                    let bytes = &board.digits.rgba()[index..index + 3];
                    let expected =
                        u32::from(bytes[0]) << 16 | u32::from(bytes[1]) << 8 | u32::from(bytes[2]);
                    assert_eq!(canvas.pixels()[(y + row) * 256 + x + column], expected);
                }
            }
        }
    }
}
