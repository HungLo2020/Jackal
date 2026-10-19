//! Passing bomb carriers (3A/3B) and decelerating attack aircraft (43).
//! Original handlers8B2E..8BA2 andB317..B408; no terrain steering is used.
use crate::{
    enemy::{aim_direction, direction_velocity},
    math::Vec2,
    weapon::Projectile,
};
#[derive(Clone, Debug)]
pub struct SupportVehicle {
    pub position: Vec2,
    pub sprite: u16,
    pub palette: u8,
    pub flips: (bool, bool),
    pub alive: bool,
    pub phase: u8,
    pub timer: u8,
    source: u8,
    heading: u8,
    speed_step: u8,
    velocity: Vec2,
    rank: u16,
    sounds: Vec<u8>,
}
impl SupportVehicle {
    pub fn new(source: u8, position: Vec2) -> Self {
        assert!(matches!(source, 58 | 59 | 67));
        Self {
            position,
            sprite: 0,
            palette: 0,
            flips: (false, false),
            alive: true,
            phase: 0,
            timer: 0,
            source,
            heading: 8,
            speed_step: 0,
            velocity: Vec2::ZERO,
            rank: 0,
            sounds: Vec::new(),
        }
    }
    pub fn drain_sound_commands(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.sounds)
    }
    pub fn set_rank(&mut self, rank: u16) {
        self.rank = rank;
    }
    pub fn tick(&mut self, target: Vec2, camera: Vec2, global_tick: u64) -> Vec<Projectile> {
        let mut shots = Vec::new();
        if !self.alive {
            return shots;
        }
        if self.phase == 0 {
            if self.source != 58 {
                let screen_x = (target.x - camera.x) as u8;
                self.position.x = camera.x
                    + f32::from(screen_x.wrapping_add(if screen_x < 128 { 32 } else { 224 }));
            }
            let from_bottom = self.position.y - camera.y >= 128.;
            self.heading = if from_bottom { 24 } else { 8 };
            self.flips.1 = if self.source == 67 {
                !from_bottom
            } else {
                from_bottom
            };
            self.sprite = if self.source == 67 { 0x9b } else { 0x4e };
            self.velocity =
                direction_velocity(self.heading, if self.source == 67 { 2.5 } else { 2. });
            if self.source != 67
                && (!(0.0..256.0).contains(&(self.position.x - camera.x))
                    || !(0.0..256.0).contains(&(self.position.y - camera.y)))
            {
                self.alive = false;
                self.sprite = 0;
            }
            self.phase = 1;
            return shots;
        }
        if self.source != 67 && self.timer & 3 == 0 {
            self.sounds.push(0x0a);
        }
        self.timer = self.timer.wrapping_add(1);
        if self.source == 67 && self.timer & 3 == 0 {
            self.sounds.push(0x0c);
        }
        if self.source == 67 {
            self.sprite =
                if self.phase == 1 { 0x9b } else { 0x9d } + u16::from(self.timer & 2 != 0);
            if self.phase == 1 && self.timer & 15 == 0 {
                self.speed_step += 1;
                self.velocity = direction_velocity(
                    self.heading,
                    [2.5, 2., 1.5, 1., 0.5, 0., 0.][usize::from(self.speed_step)],
                );
            }
            self.position += self.velocity;
            if self.timer & 31 == 0 {
                shots.push(Projectile::enemy_pattern(
                    self.position,
                    direction_velocity(aim_direction(target - self.position), 2.),
                    24 + 8 * self.rank,
                    5,
                    16,
                ));
            }
            if self.phase == 1 && self.speed_step == 6 {
                let mut direction = if self.position.x - camera.x >= 128. {
                    self.flips.0 = true;
                    12_u8
                } else {
                    4_u8
                };
                if self.heading & 16 != 0 {
                    direction = 0_u8.wrapping_sub(direction) & 31;
                }
                self.velocity = direction_velocity(direction, 1.75);
                self.phase = 2;
            }
        } else {
            self.sprite = 0x4e + u16::from(self.timer & 1 != 0);
            self.position += self.velocity;
            if matches!(self.timer, 16 | 48 | 80 | 112)
                && (0.0..256.0).contains(&(self.position.x - camera.x))
                && (0.0..256.0).contains(&(self.position.y - camera.y))
            {
                shots.push(Projectile::enemy_bomb(
                    self.position,
                    aim_direction(target - self.position),
                    self.source == 58,
                    global_tick,
                ));
            }
        }
        if !(0.0..512.0).contains(&self.position.x)
            || !(-64.0..320.0).contains(&(self.position.y - camera.y))
        {
            self.alive = false;
            self.sprite = 0;
        }
        shots
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passing_support_vehicles_match_six_original_600_tick_runs() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/support_vehicle.json"))
                .unwrap();
        for capture in captures.as_array().unwrap() {
            let source = capture["source_type"].as_u64().unwrap() as u8;
            let y = capture["start_y"].as_f64().unwrap() as f32;
            let mut actor = SupportVehicle::new(source, Vec2::new(80., y));
            for sample in capture["samples"].as_array().unwrap() {
                let tick = sample["tick"].as_u64().unwrap();
                let shots = actor.tick(Vec2::new(80., 160.), Vec2::ZERO, tick);
                assert_eq!(
                    u64::from(actor.sprite),
                    sample["sprite"].as_u64().unwrap(),
                    "source{source} y{y} tick{tick} sprite"
                );
                assert_eq!(
                    actor.position.x,
                    sample["x"].as_f64().unwrap() as f32,
                    "source{source} y{y} tick{tick} x"
                );
                assert_eq!(
                    actor.position.y,
                    sample["y"].as_f64().unwrap() as f32,
                    "source{source} y{y} tick{tick} y"
                );
                assert_eq!(
                    shots.len(),
                    sample["shots"].as_array().unwrap().len(),
                    "source{source} y{y} tick{tick} shots"
                );
                if actor.alive {
                    assert_eq!(
                        u64::from(u8::from(actor.flips.0) * 64 + u8::from(actor.flips.1) * 128),
                        sample["flips"].as_u64().unwrap(),
                        "source{source} y{y} tick{tick} flips"
                    );
                }
            }
        }
    }
}
