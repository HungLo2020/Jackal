//! Original bottom-of-screen score and reserve-life display.
use crate::{
    player::Player,
    renderer::{Canvas, Rect, Texture},
};
use std::path::Path;

pub struct HudPiece<'a> {
    pub texture: &'a Texture,
    pub source: Rect,
    pub destination: Rect,
}

pub struct Hud {
    font: Texture,
}
impl Hud {
    pub fn load(root: &Path) -> Result<Self, String> {
        let image = image::open(root.join("fonts/hud.png"))
            .map_err(|e| e.to_string())?
            .into_rgba8();
        if image.dimensions() != (96, 16) {
            return Err("HUD font must contain twelve 8 by 16 glyphs".into());
        }
        Ok(Self {
            font: Texture::from_rgba(96, 16, image.into_raw())?,
        })
    }
    #[cfg(test)]
    fn glyph(&self, canvas: &mut Canvas, glyph: u32, x: i32, y: i32) {
        canvas.draw_region(
            &self.font,
            Rect::new(glyph as i32 * 8, 0, 8, 16),
            Rect::new(x, y, 8, 16),
            false,
            false,
        );
    }
    /// Individual 8×16 pieces in original submission order, for shared sprite budgeting.
    pub fn pieces(&self, players: &[Player; 2], tick: u64) -> Vec<HudPiece<'_>> {
        let index = if players[1].enabled && tick & 128 != 0 {
            1
        } else {
            0
        };
        let player = &players[index];
        if !player.enabled {
            return Vec::new();
        }
        let x = 16 + index as i32 * 144;
        let mut pieces = Vec::with_capacity(10);
        let mut glyph = |glyph: u32, x, y| {
            pieces.push(HudPiece {
                texture: &self.font,
                source: Rect::new(glyph as i32 * 8, 0, 8, 16),
                destination: Rect::new(x, y, 8, 16),
            })
        };
        glyph(index as u32 + 1, x, 201);
        glyph(10, x + 8, 201);
        let score = format!("{:06}", player.score.min(999_999));
        for (column, digit) in score.bytes().enumerate() {
            glyph((digit - b'0') as u32, x + 24 + column as i32 * 8, 201);
        }
        glyph(10 + index as u32, x + 28, 217);
        glyph(player.lives.saturating_sub(1).min(9) as u32, x + 38, 217);
        pieces
    }
    pub fn draw(&self, canvas: &mut Canvas, players: &[Player; 2], tick: u64) {
        for piece in self.pieces(players, tick) {
            canvas.draw_region(piece.texture, piece.source, piece.destination, false, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;
    #[test]
    fn original_hud_uses_bottom_overlay_and_reserve_lives() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let hud = Hud::load(&root).unwrap();
        let players = [
            Player::new(Vec2::new(0.0, 0.0), true),
            Player::new(Vec2::new(0.0, 0.0), false),
        ];
        let mut canvas = Canvas::new(256, 240);
        canvas.clear(0x123456);
        hud.draw(&mut canvas, &players, 0);
        assert!(canvas.pixels()[..200 * 256]
            .iter()
            .all(|&pixel| pixel == 0x123456));
        let mut expected = Canvas::new(256, 240);
        expected.clear(0x123456);
        hud.glyph(&mut expected, 4, 54, 217);
        for y in 217..233 {
            for x in 54..62 {
                assert_eq!(canvas.pixels()[y * 256 + x], expected.pixels()[y * 256 + x]);
            }
        }
    }
    #[test]
    fn two_player_hud_preserves_original_128_frame_alternation() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let hud = Hud::load(&root).unwrap();
        let players = [
            Player::new(Vec2::new(0.0, 0.0), true),
            Player::new(Vec2::new(0.0, 0.0), true),
        ];
        let mut even = Canvas::new(256, 240);
        let mut odd = Canvas::new(256, 240);
        hud.draw(&mut even, &players, 128);
        hud.draw(&mut odd, &players, 127);
        assert!(even.pixels()[201 * 256..217 * 256]
            .chunks(256)
            .all(|row| row[..144].iter().all(|&p| p == 0)));
        assert!(odd.pixels()[201 * 256..217 * 256]
            .chunks(256)
            .all(|row| row[144..].iter().all(|&p| p == 0)));
        assert!(even.pixels().iter().any(|&p| p != 0));
        assert!(odd.pixels().iter().any(|&p| p != 0));
    }
}
