//! Recovered per-actor states for the background gun towers, retracting statues,
//! and industrial-base guard launchers. Timers are unsigned original counters.
use crate::{
    enemy::{aim_direction, direction_velocity},
    math::Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossActorKind {
    Tower,
    Statue,
    Guard,
}
#[derive(Clone, Copy, Debug)]
pub enum BossAttack {
    Bullet { direction: u8, velocity: Vec2 },
    Shell,
    Spawn(u8),
}
#[derive(Clone, Debug)]
pub struct BossActor {
    pub kind: BossActorKind,
    pub state: u8,
    pub sprite: u16,
    pub vulnerable: bool,
    pub patch: Option<u8>,
    pub timer: u8,
    pub fire_timer: u8,
    slot: u8,
    variant: u8,
    initialized: bool,
    repeat: u8,
    patch_step: u8,
    phase_tick: u8,
    shots_fired: u8,
    pub deployed_vehicles: u8,
    openings: u8,
    vehicle_limit: u8,
}
impl BossActor {
    pub fn new(kind: BossActorKind, slot: u8, variant: u8) -> Self {
        Self {
            kind,
            state: 0,
            sprite: 0,
            vulnerable: false,
            patch: None,
            timer: 0,
            fire_timer: 0,
            slot,
            variant,
            initialized: false,
            repeat: 0,
            patch_step: 0,
            phase_tick: 0,
            shots_fired: 0,
            deployed_vehicles: 0,
            openings: 0,
            vehicle_limit: 4,
        }
    }
    pub fn set_slot(&mut self, slot: u8) {
        self.slot = slot;
    }
    pub fn tick(&mut self, delta: Vec2, remaining_parts: usize) -> Vec<BossAttack> {
        let mut attacks = Vec::new();
        if !self.initialized {
            self.initialized = true;
            self.state = 1;
            match self.kind {
                BossActorKind::Tower => {
                    self.fire_timer = 192;
                    self.repeat = 192;
                    self.vulnerable = true;
                }
                BossActorKind::Statue => {
                    self.sprite = 0x63;
                    self.timer = 8;
                    self.fire_timer = self.slot.wrapping_mul(4);
                    self.repeat = self.slot.wrapping_mul(2).wrapping_add(16);
                }
                BossActorKind::Guard => {
                    self.fire_timer = [48, 96, 144, 192][usize::from(self.variant & 3)];
                    self.patch_step = 20;
                }
            }
            return attacks;
        }
        match self.kind {
            BossActorKind::Tower => {
                self.fire_timer = self.fire_timer.wrapping_sub(1);
                match self.state {
                    1 => {
                        self.patch = Some(11);
                        if self.fire_timer == 0 {
                            self.fire_timer = 24;
                            self.state = 2;
                        }
                    }
                    2 => {
                        self.patch = Some(if self.fire_timer & 2 != 0 { 8 } else { 9 });
                        if self.fire_timer == 0 {
                            self.fire_timer = 24;
                            self.state = 3;
                        }
                    }
                    3 => {
                        self.patch = Some(10);
                        if self.fire_timer == 0 {
                            self.fire_timer = self.repeat;
                            self.state = 1;
                            attacks.push(BossAttack::Shell);
                        }
                    }
                    _ => unreachable!(),
                }
            }
            BossActorKind::Statue => match self.state {
                1 => {
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer >= 128 && self.timer != 192 && self.timer & 15 == 0 {
                        self.sprite += 1;
                        self.vulnerable = true;
                    }
                    if self.timer == 192 {
                        self.state = 2;
                    }
                }
                2 => {
                    self.fire_timer = self.fire_timer.wrapping_sub(1);
                    if self.fire_timer == 0 {
                        self.fire_timer = self.repeat;
                        let aim = aim_direction(delta);
                        for offset in [28, 30, 0, 2, 4] {
                            let direction = (aim + offset) & 31;
                            attacks.push(BossAttack::Bullet {
                                direction,
                                velocity: direction_velocity(direction, 1.75),
                            });
                        }
                        self.timer = 16;
                        self.state = 3;
                    }
                }
                3 => {
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer >= 128 && self.timer != 192 && self.timer & 15 == 0 {
                        self.sprite -= 1;
                    }
                    if self.timer == 192 {
                        self.vulnerable = false;
                        self.timer = if remaining_parts < 2 { 1 } else { 80 };
                        if remaining_parts < 2 {
                            self.fire_timer = 9;
                        }
                        self.state = 1;
                    }
                }
                _ => unreachable!(),
            },
            BossActorKind::Guard => match self.state {
                1 => {
                    self.patch = Some(self.patch_step);
                    self.fire_timer = self.fire_timer.wrapping_sub(1);
                    if self.fire_timer == 0 {
                        self.phase_tick = 0;
                        self.openings = self.openings.wrapping_add(1);
                        if self.openings & 63 == 0 {
                            self.vehicle_limit = self.vehicle_limit.wrapping_add(1);
                        }
                        self.state = 2;
                    }
                }
                2 => {
                    self.phase_tick = self.phase_tick.wrapping_add(1);
                    if self.phase_tick & 7 == 0 {
                        self.patch = Some(self.patch_step);
                        self.patch_step -= 1;
                        if self.patch_step < 17 {
                            self.patch_step += 1;
                            self.phase_tick = 0;
                            self.sprite = 255;
                            self.vulnerable = true;
                            self.state = 3;
                        }
                    }
                }
                3 => {
                    self.phase_tick = self.phase_tick.wrapping_add(1);
                    if self.phase_tick >= 32 {
                        self.patch = Some(self.patch_step);
                        if self.deployed_vehicles < self.vehicle_limit {
                            let source = [35, 36, 36, 35][usize::from(self.variant & 3)];
                            self.variant = (self.variant + 1) & 3;
                            self.shots_fired = self.shots_fired.wrapping_add(1);
                            attacks.push(BossAttack::Spawn(source));
                        }
                        self.phase_tick = 0;
                        self.sprite = 0;
                        self.vulnerable = false;
                        self.state = 4;
                    }
                }
                4 => {
                    self.phase_tick = self.phase_tick.wrapping_add(1);
                    if self.phase_tick & 7 == 0 {
                        self.patch = Some(self.patch_step);
                        self.patch_step += 1;
                        if self.patch_step >= 21 {
                            self.patch_step -= 1;
                            self.state = 1;
                        }
                    }
                }
                _ => unreachable!(),
            },
        }
        attacks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovered_actor_sequences_match_original_seven_hundred_ticks_each() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/boss_sequences.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let source = capture["source_type"].as_u64().unwrap();
            if source == 64 {
                continue;
            }
            let kind = match source {
                24 => BossActorKind::Tower,
                146 => BossActorKind::Statue,
                49 => BossActorKind::Guard,
                _ => unreachable!(),
            };
            let mut actor = BossActor::new(kind, 0, 0);
            for expected in capture["samples"].as_array().unwrap() {
                let attacks = actor.tick(Vec2::new(0., 80.), 6);
                let tick = expected["tick"].as_u64().unwrap();
                assert_eq!(
                    u64::from(actor.state),
                    expected["state"].as_u64().unwrap(),
                    "type{source} tick{tick} state"
                );
                assert_eq!(
                    u64::from(actor.sprite),
                    expected["sprite"].as_u64().unwrap(),
                    "type{source} tick{tick} sprite"
                );
                assert_eq!(
                    u64::from(actor.fire_timer),
                    expected["fire_timer"].as_u64().unwrap(),
                    "type{source} tick{tick} timer"
                );
                let types: Vec<u64> = attacks
                    .iter()
                    .map(|a| match a {
                        BossAttack::Bullet { .. } => 54,
                        BossAttack::Shell => 57,
                        BossAttack::Spawn(t) => u64::from(*t),
                    })
                    .collect();
                let expected_types: Vec<u64> = expected["shots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect();
                assert_eq!(types, expected_types, "type{source} tick{tick} attacks");
            }
        }
    }
}

/// Stage four's aircraft approaches from above, pauses to launch three pods,
/// then retreats and repeats. Speeds are original fixed-point parameter table.
#[derive(Clone, Debug)]
pub struct Aircraft {
    pub position: Vec2,
    pub sprite: u16,
    pub state: u8,
    pub timer: u8,
    pub pod_index: u8,
    pub pod_direction: u8,
    velocity: Vec2,
    speed_index: u8,
    camera_y: f32,
    initialized: bool,
}
impl Aircraft {
    pub fn new(position: Vec2, camera_y: f32) -> Self {
        Self {
            position,
            sprite: 0,
            state: 0,
            timer: 0,
            pod_index: 0,
            pod_direction: 0,
            velocity: Vec2::default(),
            speed_index: 0,
            camera_y,
            initialized: false,
        }
    }
    pub fn tick(&mut self, target: Vec2) -> Vec<BossAttack> {
        const SPEEDS: [f32; 10] = [2.5, 2., 1.5, 1.25, 1., 0.75, 0.5, 0.25, 0.125, 0.];
        let mut attacks = Vec::new();
        if !self.initialized {
            self.initialized = true;
            self.state = 1;
            self.timer = 128;
            return attacks;
        }
        match self.state {
            1 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer == 0 {
                    let screen_x = target.x % 256.;
                    self.position.x = target.x + if screen_x < 128. { 32. } else { -32. };
                    self.position.y = self.camera_y - 32. + self.position.y.rem_euclid(1.);
                    self.timer = 144;
                    self.speed_index = 0;
                    self.velocity = Vec2::new(0., SPEEDS[0]);
                    self.state = 2;
                }
            }
            2 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer & 15 == 0 {
                    self.speed_index += 1;
                    self.velocity = Vec2::new(0., SPEEDS[usize::from(self.speed_index)]);
                }
                self.sprite = 0x7e + u16::from(self.timer & 2 != 0);
                self.position += self.velocity;
                if self.timer == 0 {
                    self.timer = 64;
                    self.pod_index = 0;
                    self.velocity.x = 0.25;
                    self.pod_direction = if self.position.x < 256. { 0 } else { 16 };
                    self.state = 3;
                } else if self.timer & 31 == 0 {
                    let direction = aim_direction(target - self.position);
                    attacks.push(BossAttack::Bullet {
                        direction,
                        velocity: direction_velocity(direction, 1.75),
                    });
                }
            }
            3 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer & 3 == 0 {
                    self.velocity.x = -self.velocity.x;
                }
                self.sprite = 0x7e + u16::from(self.timer & 2 != 0);
                self.position += self.velocity;
                if self.timer == 0 {
                    self.speed_index -= 1;
                    self.velocity = Vec2::new(0., -SPEEDS[usize::from(self.speed_index)]);
                    self.state = 4;
                } else {
                    if self.timer & 31 == 0 {
                        let direction = aim_direction(target - self.position);
                        attacks.push(BossAttack::Bullet {
                            direction,
                            velocity: direction_velocity(direction, 1.75),
                        });
                    }
                    if self.timer & 15 == 0 {
                        attacks.push(BossAttack::Spawn(66));
                        self.pod_index += 1;
                    }
                }
            }
            4 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer & 15 == 0 {
                    self.speed_index = self.speed_index.saturating_sub(1);
                    self.velocity = Vec2::new(0., -SPEEDS[usize::from(self.speed_index)]);
                }
                self.sprite = 0x7e + u16::from(self.timer & 2 != 0);
                self.position += self.velocity;
                if self.position.y < self.camera_y {
                    self.velocity = Vec2::default();
                    self.timer = 128;
                    self.state = 1;
                } else if self.timer & 31 == 0 {
                    let direction = aim_direction(target - self.position);
                    attacks.push(BossAttack::Bullet {
                        direction,
                        velocity: direction_velocity(direction, 1.75),
                    });
                }
            }
            _ => unreachable!(),
        }
        attacks
    }
}

#[cfg(test)]
mod aircraft_tests {
    use super::*;
    #[test]
    fn aircraft_matches_original_positions_and_attack_timing() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/boss_sequences.json")).unwrap();
        let capture = captures
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["source_type"] == 64)
            .unwrap();
        let mut aircraft = Aircraft::new(Vec2::new(80., 80.), 0.);
        for expected in capture["samples"].as_array().unwrap() {
            let attacks = aircraft.tick(Vec2::new(80., 160.));
            let tick = expected["tick"].as_u64().unwrap();
            assert_eq!(
                u64::from(aircraft.state),
                expected["state"].as_u64().unwrap() & 127,
                "tick{tick} state"
            );
            assert_eq!(
                u64::from(aircraft.sprite),
                expected["sprite"].as_u64().unwrap(),
                "tick{tick} sprite"
            );
            assert_eq!(
                u64::from(aircraft.timer),
                expected["timer"].as_u64().unwrap(),
                "tick{tick} timer"
            );
            assert_eq!(
                f64::from(aircraft.position.x),
                expected["x"].as_f64().unwrap(),
                "tick{tick} x"
            );
            assert_eq!(
                f64::from(aircraft.position.y),
                expected["y"].as_f64().unwrap(),
                "tick{tick} y"
            );
            let mut types: Vec<u64> = attacks
                .iter()
                .map(|a| match a {
                    BossAttack::Bullet { .. } => 54,
                    BossAttack::Shell => 57,
                    BossAttack::Spawn(t) => u64::from(*t),
                })
                .collect();
            let mut expected_types: Vec<u64> = expected["shots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect();
            types.sort();
            expected_types.sort();
            assert_eq!(types, expected_types, "tick{tick} attacks");
        }
    }
}
