//! Ordinary actor death handlers F84F/F866/F88D and turret exception B8FA.
use super::Enemy;
use crate::{actor_pool::ActorHandle, assets::Assets, math::Vec2};
#[derive(Clone, Copy, Debug)]
enum Phase {
    Prelude,
    Start,
    Explosion,
    Spent,
}
#[derive(Clone, Debug)]
pub struct EnemyDeath {
    pub actor_handle: Option<ActorHandle>,
    pub position: Vec2,
    pub sprite: u16,
    pub palette: u8,
    pub flips: (bool, bool),
    pub alive: bool,
    source: u8,
    stage: usize,
    phase: Phase,
    timer: u8,
}
impl EnemyDeath {
    pub fn new(enemy: &Enemy, assets: &Assets) -> Self {
        Self {
            actor_handle: enemy.actor_handle,
            position: enemy.position,
            sprite: enemy.sprite(assets).unwrap_or(0),
            palette: enemy.palette_override().unwrap_or(0),
            flips: enemy.sprite_flips(),
            alive: true,
            source: enemy.definition,
            stage: enemy.stage,
            phase: if enemy.definition == 18 {
                Phase::Prelude
            } else {
                Phase::Start
            },
            timer: 0,
        }
    }
    pub fn source_type(&self) -> u8 {
        self.source
    }
    pub fn terrain_patch(&self) -> Option<u8> {
        matches!(self.source, 33 | 34).then_some(7)
    }
    /// Each call is one original actor update, including the initialization tick.
    pub fn tick(&mut self, camera: Vec2, page: u8, boss_remaining: u8) -> Vec<u8> {
        if !self.alive {
            return Vec::new();
        }
        let screen = self.position - camera;
        let visible = (0. ..256.).contains(&screen.x) && (0. ..256.).contains(&screen.y);
        let mut sounds = Vec::new();
        match self.phase {
            Phase::Prelude => {
                if boss_remaining == 1 {
                    sounds.push(0x59);
                }
                self.phase = Phase::Start;
            }
            Phase::Start => {
                self.sprite = 25;
                self.palette = 3;
                self.timer = 24;
                self.phase = Phase::Explosion;
                let audible = if self.source == 5 {
                    !(self.stage == 5 && page == 13)
                } else {
                    visible
                };
                if audible {
                    sounds.push(0x17);
                }
            }
            Phase::Explosion => {
                self.timer -= 1;
                if self.timer == 0 {
                    self.sprite = 0;
                    if matches!(self.source, 33 | 34) {
                        self.phase = Phase::Spent;
                    } else {
                        self.alive = false;
                        self.palette = 0;
                    }
                } else if self.timer.is_multiple_of(8) {
                    self.sprite += 1;
                }
            }
            Phase::Spent => {}
        }
        if self.position.x < 0. || self.position.x >= 512. || screen.y < -64. || screen.y >= 320. {
            self.alive = false;
            self.sprite = 0;
            self.palette = 0;
        }
        sounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn death_animation_sound_and_retained_background_match_original_handlers() {
        let assets =
            Assets::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/enemy_deaths.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let source = capture["source"].as_u64().unwrap() as u8;
            let mut enemy = Enemy::new(
                assets.entity(source),
                Vec2::new(capture["x"].as_f64().unwrap() as f32, 80.),
                super::super::Behavior::Vehicle,
            );
            enemy.stage = capture["stage"].as_u64().unwrap() as usize;
            enemy.current_sprite = Some(76);
            enemy.current_palette = Some(2);
            enemy.sprite_flip_x = true;
            let mut death = EnemyDeath::new(&enemy, &assets);
            for sample in capture["samples"].as_array().unwrap() {
                let sounds = death.tick(
                    Vec2::ZERO,
                    capture["page"].as_u64().unwrap() as u8,
                    capture["boss_remaining"].as_u64().unwrap() as u8,
                );
                let expected_sounds: Vec<u8> = sample["sounds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u8)
                    .collect();
                assert_eq!(
                    (death.sprite, death.palette, death.alive, sounds),
                    (
                        sample["sprite"].as_u64().unwrap() as u16,
                        sample["palette"].as_u64().unwrap() as u8,
                        sample["alive"].as_bool().unwrap(),
                        expected_sounds
                    ),
                    "source {source} tick {} x {} stage {}",
                    sample["tick"],
                    capture["x"],
                    capture["stage"]
                );
                assert_eq!(death.flips, (true, false));
            }
        }
    }
}
