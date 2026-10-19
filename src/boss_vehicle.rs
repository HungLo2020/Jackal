//! Shared armored-vehicle maneuver states recovered from bank6 814D..8260.
use crate::{
    enemy::{aim_direction, direction_velocity},
    math::Vec2,
};
#[derive(Clone, Debug)]
pub struct ArmoredVehicle {
    pub position: Vec2,
    pub sprite: u16,
    pub flips: (bool, bool),
    pub direction: u8,
    pub state: u8,
    pub timer: u8,
    source: u8,
    stage: usize,
    slot: u8,
    velocity: Vec2,
    turn_tick: u8,
    fire_tick: u8,
    initialized: bool,
    entry_from_bottom: bool,
}
impl ArmoredVehicle {
    pub fn new(source: u8, slot: u8, position: Vec2) -> Self {
        Self {
            position,
            sprite: 0,
            flips: (false, false),
            direction: 8,
            state: 0,
            timer: 0,
            source,
            stage: 0,
            slot,
            velocity: Vec2::default(),
            turn_tick: 0,
            fire_tick: 32,
            initialized: false,
            entry_from_bottom: position.y >= 128.,
        }
    }
    pub fn set_slot(&mut self, slot: u8) {
        self.slot = slot;
    }
    pub fn set_stage(&mut self, stage: usize) {
        self.stage = stage;
    }
    pub fn set_entry_from_bottom(&mut self, value: bool) {
        self.entry_from_bottom = value;
    }
    pub fn probe_position(&self) -> Vec2 {
        crate::enemy::infantry::lookahead(self.position, self.direction, self.state != 2)
    }
    pub fn collision_position(&self) -> Vec2 {
        self.position + self.velocity * 2.
    }
    fn speed(&self) -> f32 {
        match self.source {
            10 => 1.,
            11 | 35 => 0.75,
            14 => 0.5,
            15 => 1.5,
            _ => 0.625,
        }
    }
    fn frame(&mut self, special: bool) {
        let base = match self.source {
            10 => 0x33,
            11 | 14 | 35 => 0x44,
            15 => {
                if self.stage == 5 {
                    0xb5
                } else {
                    0x3d
                }
            }
            _ => 0x22,
        };
        let entry: u8 =
            [0, 1, 2, 0x41, 0x40, 0xc1, 0x82, 0x81][usize::from(((self.direction + 1) & 31) / 4)];
        self.sprite = base + u16::from(entry & 15);
        self.flips = (entry & 64 != 0, entry & 128 != 0);
        if special && self.source == 10 && self.sprite == 0x34 && self.flips.1 {
            self.sprite += 5;
            self.flips.1 = false;
        }
    }
    pub fn tick(&mut self, target: Vec2, terrain_blocked: bool, actor_blocked: bool) -> bool {
        if !self.initialized {
            self.initialized = true;
            if matches!(self.source, 7 | 11 | 14 | 15) {
                self.direction = (((aim_direction(target - self.position) + 1) & 31) / 4) * 4;
                self.state = 2;
                self.fire_tick = if self.source == 15 { 64 } else { 32 };
                self.frame(false);
                return false;
            }

            self.state = 1;
            self.timer = match self.source {
                10 => {
                    if self.entry_from_bottom {
                        24
                    } else {
                        48
                    }
                }
                35 => 56,
                _ => 64,
            };
            if self.source == 10 && self.entry_from_bottom {
                self.direction = 24;
            }
            self.velocity = direction_velocity(self.direction, self.speed());
            self.frame(true);
            return false;
        }
        let turn_mask = match self.source {
            7 | 36 => 31,
            15 => 7,
            14 => 127,
            _ => 15,
        };
        let mut fire = false;
        match self.state {
            1 => {
                self.position += self.velocity;
                self.timer = self.timer.wrapping_sub(1);
                if self.timer == 0 {
                    if terrain_blocked || actor_blocked {
                        self.timer = if self.source == 10 { 8 } else { 16 };
                    } else {
                        self.state = 2;
                    }
                } else if self.source != 10 && self.timer & 15 == 0 {
                    fire = true;
                }
            }
            2 => {
                let wanted = aim_direction(target - self.position);
                let delta = wanted.wrapping_sub(self.direction);
                let mut ready = delta == 0 && !terrain_blocked;
                if !ready {
                    self.turn_tick = self.turn_tick.wrapping_sub(1);
                    if self.turn_tick & turn_mask == 0 {
                        self.direction = if delta & 16 == 0 {
                            self.direction.wrapping_add(4)
                        } else {
                            self.direction.wrapping_sub(4)
                        } & 31;
                        ready = true;
                    }
                }
                self.frame(true);
                self.position += self.velocity;
                if ready {
                    self.timer = match self.source {
                        10 => 40,
                        15 => 32,
                        14 => 48,
                        _ => 80,
                    };
                    self.velocity = direction_velocity(self.direction, self.speed());
                    self.state = 3;
                }
            }
            3 => {
                let blocked = terrain_blocked || actor_blocked;
                if blocked {
                    self.velocity = Vec2::default();
                    self.turn_tick = self.turn_tick.wrapping_sub(1);
                    if self.turn_tick & turn_mask == 0 {
                        self.direction =
                            self.direction
                                .wrapping_add(if self.slot & 1 == 0 { 4 } else { 252 })
                                & 31;
                    }
                }
                self.frame(true);
                self.position += self.velocity;
                self.velocity = direction_velocity(self.direction, self.speed());
                if !blocked {
                    self.state = 4;
                }
            }
            4 => {
                if self.source == 15 {
                    let d = Vec2::new(
                        target.x.floor() - self.position.x.floor(),
                        target.y.floor() - self.position.y.floor(),
                    );
                    if d.x.abs() + d.y.abs() < 32. {
                        self.velocity = direction_velocity(self.direction, 1.);
                    }
                }
                self.timer = self.timer.wrapping_sub(1);
                self.frame(true);
                if self.source == 10 && self.timer & 8 != 0 {
                    self.sprite += 3;
                }
                let blocked = self.timer == 0 || terrain_blocked || actor_blocked;
                if blocked {
                    self.velocity = Vec2::default();
                    if self.source == 10 {
                        self.frame(false);
                    }
                }
                self.position += self.velocity;
                if blocked {
                    self.state = 2;
                    fire = self.source == 14;
                } else if self.source == 14 {
                    // This variant fires only when its movement ends.
                } else if self.source == 10 {
                    fire = self.timer & 15 == 0;
                } else {
                    self.fire_tick = self.fire_tick.wrapping_sub(1);
                    if self.fire_tick == 0 {
                        self.fire_tick = if self.source == 15 { 64 } else { 32 };
                        fire = true;
                    }
                }
            }
            _ => unreachable!(),
        }
        fire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn armored_vehicle_clear_ground_sequences_match_original() {
        let captures: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/boss_vehicle_sequences.json"
        ))
        .unwrap();
        for capture in captures.as_array().unwrap() {
            let source = capture["source_type"].as_u64().unwrap() as u8;
            let mut vehicle = ArmoredVehicle::new(source, 0, Vec2::new(80., 80.));
            for e in capture["samples"].as_array().unwrap() {
                let fire = vehicle.tick(Vec2::new(80., 160.), false, false);
                let tick = e["tick"].as_u64().unwrap();
                assert_eq!(
                    u64::from(vehicle.state),
                    e["state"].as_u64().unwrap() & 127,
                    "type{source} tick{tick} state"
                );
                assert_eq!(
                    u64::from(vehicle.sprite),
                    e["sprite"].as_u64().unwrap(),
                    "type{source} tick{tick} sprite"
                );
                assert_eq!(
                    f64::from(vehicle.position.x),
                    e["x"].as_f64().unwrap(),
                    "type{source} tick{tick} x"
                );
                assert_eq!(
                    f64::from(vehicle.position.y),
                    e["y"].as_f64().unwrap(),
                    "type{source} tick{tick} y"
                );
                assert_eq!(
                    usize::from(fire),
                    e["shots"].as_array().unwrap().len(),
                    "type{source} tick{tick} firing"
                );
            }
        }
    }
}
