//! Shared scrolling and screen boundaries recovered from the original jeep logic.
use crate::{assets::Level, math::Vec2, player::Player, renderer::Camera};

#[derive(Default)]
pub struct CameraController {
    /// Only the most recent 64 pixels of voluntary forward travel can be revisited.
    backtrack: u8,
}
impl CameraController {
    pub fn reset(&mut self) {
        self.backtrack = 0;
    }
    pub fn backtrack_pixels(&self) -> u8 {
        self.backtrack
    }
    pub fn update(
        &mut self,
        camera: &mut Camera,
        players: &mut [Player; 2],
        level: &Level,
        boss: bool,
        forced: bool,
    ) {
        let minimum_y = (level.start_camera_y() - (level.boss.distance - 240.)).max(0.);
        if forced {
            if !boss {
                let old_y = camera.y;
                camera.y = (camera.y - 1.).max(minimum_y);
                for player in players.iter_mut().filter(|p| p.enabled && p.lives > 0) {
                    // Once a jeep reaches screen row 208, the approach carries it
                    // forward with the camera instead of pushing it offscreen.
                    if player.position.y - old_y >= 208. {
                        player.position.y += camera.y - old_y;
                    }
                }
            }
            return;
        }
        let present = |p: &Player| p.enabled && p.lives > 0;
        if !players.iter().any(present) {
            return;
        }
        let mut screen = [Vec2::new(128., 128.); 2];
        for (i, player) in players.iter().enumerate() {
            if present(player) {
                screen[i] = player.position - Vec2::new(camera.x, camera.y);
            }
        }
        let maximum_x = (level.width() - 257.).max(0.);
        let horizontal;
        let vertical;
        if !players[1].enabled {
            horizontal = if screen[0].x < 64. && camera.x > 0. {
                -1.
            } else if screen[0].x >= 192. && camera.x < maximum_x {
                1.
            } else {
                0.
            };
            vertical = if screen[0].y < 96. {
                -1.
            } else if screen[0].y >= 192. && self.backtrack > 0 {
                1.
            } else {
                0.
            };
        } else {
            let left = [0, 1].into_iter().any(|i| {
                players[i].last_motion.vector().x < 0.
                    && screen[i].x < 64.
                    && screen[1 - i].x < 192.
            });
            let right = [0, 1].into_iter().any(|i| {
                players[i].last_motion.vector().x >= 0.
                    && screen[i].x >= 193.
                    && screen[1 - i].x >= 65.
            });
            horizontal = if left && camera.x > 0. {
                -1.
            } else if right && camera.x < maximum_x {
                1.
            } else {
                0.
            };
            let forward = [0, 1].into_iter().any(|i| {
                players[i].last_motion.vector().y <= 0.
                    && screen[i].y < 96.
                    && screen[1 - i].y < 176.
            });
            let backward = [0, 1].into_iter().any(|i| {
                players[i].last_motion.vector().y > 0.
                    && screen[i].y >= 177.
                    && screen[1 - i].y >= 97.
            });
            vertical = if forward {
                -1.
            } else if backward && self.backtrack > 0 {
                1.
            } else {
                0.
            };
        }
        camera.x = (camera.x + horizontal).clamp(0., maximum_x);
        if !boss {
            let old = camera.y;
            camera.y = (camera.y + vertical).clamp(minimum_y, level.start_camera_y());
            if camera.y < old {
                self.backtrack = (self.backtrack + 1).min(64);
            }
            if camera.y > old {
                self.backtrack = self.backtrack.saturating_sub(1);
            }
        }
        for player in players.iter_mut().filter(|p| present(p)) {
            player.position.x = player.position.x.clamp(camera.x + 16., camera.x + 240.);
            player.position.y = player.position.y.clamp(camera.y + 48., camera.y + 224.);
        }
    }
}
