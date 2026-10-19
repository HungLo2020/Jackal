//! Original headquarters actors (bank 6, B4A2 and B6B2).
//! Explicit unsigned counters retain the original turn and firing cadence.
use crate::{boss::BossSpawn, math::Vec2};

fn adc(a: u8, b: u8, carry: &mut bool) -> u8 {
    let sum = u16::from(a) + u16::from(b) + u16::from(*carry);
    *carry = sum > 255;
    sum as u8
}

#[derive(Clone, Debug, Default)]
pub struct HeadquartersCenter {
    pub state: u8,
    pub sprite: u16,
    pub palette: u8,
    pub fire_timer: u8,
    tank_timer: u8,
    tank_period: u8,
    blink: u8,
}
impl HeadquartersCenter {
    pub fn tick(
        &mut self,
        position: Vec2,
        health: u16,
        clock: u8,
        player_y: [u8; 2],
        camera_y: f32,
    ) -> Option<BossSpawn> {
        if self.state == 0 {
            self.fire_timer = health as u8 & 63;
            self.tank_period = clock | 192;
            self.tank_timer = self.tank_period;
            self.state = 1;
            return None;
        }
        self.tank_timer = self.tank_timer.wrapping_sub(1);
        let spawn = if self.tank_timer == 0 {
            self.tank_timer = self.tank_period;
            let mut carry = position.y - camera_y <= 239.;
            let mut value = adc(clock, self.tank_period, &mut carry);
            value = adc(value, player_y[0], &mut carry);
            value = adc(value, player_y[1], &mut carry);
            let offset = i16::from(value >> 1) + if value & 1 == 0 { -256 } else { 0 };
            Some(BossSpawn {
                source_type: 7,
                position: [position.x + f32::from(offset), camera_y + 239.],
                direction: 0,
                variant: 0,
            })
        } else {
            None
        };
        self.sprite = 0;
        if self.fire_timer != 0 {
            self.fire_timer -= 1;
        } else {
            self.sprite = 0xd1;
            self.blink = self.blink.wrapping_add(1);
            if self.blink & 7 == 0 {
                self.fire_timer = health as u8 & 63;
            }
            self.palette = (self.palette + 1) & 3;
        }
        spawn
    }
}

#[derive(Clone, Debug)]
pub struct HeadquartersGuard {
    pub state: u8,
    pub sprite: u16,
    pub flips: (bool, bool),
    pub direction: u8,
    pub timer: u8,
    pub turn_timer: u8,
    heading: u8,
    missile_lifetime: u8,
}
impl Default for HeadquartersGuard {
    fn default() -> Self {
        Self {
            state: 0,
            sprite: 0,
            flips: (false, false),
            direction: 0,
            timer: 0,
            turn_timer: 0,
            heading: 8,
            missile_lifetime: 0,
        }
    }
}
impl HeadquartersGuard {
    fn draw(&mut self) {
        let entry =
            [0_u8, 1, 2, 0x41, 0x40, 0xc1, 0x82, 0x81][usize::from(((self.heading + 1) & 31) / 4)];
        self.sprite = 0xca + u16::from(entry & 15);
        self.flips = (entry & 64 != 0, entry & 128 != 0);
    }
    pub fn tick(&mut self, position: Vec2, clock: u8, player_x: [u8; 2]) -> Option<BossSpawn> {
        match self.state {
            0 => {
                self.draw();
                self.turn_timer = 7;
                self.state = 1;
            }
            1 => {
                self.draw();
                let mut carry = clock & 8 != 0;
                let mut index = adc(clock >> 4, clock, &mut carry);
                index = adc(index, player_x[0], &mut carry);
                index = adc(index, player_x[1], &mut carry);
                self.direction = [12, 4, 12, 8, 12, 4, 4, 8][usize::from(index & 7)];
                self.missile_lifetime = [56, 84, 84, 68, 84, 56, 84, 68][usize::from(index & 7)];
                self.state = 2;
            }
            2 => {
                self.draw();
                let delta = self.direction.wrapping_sub(self.heading);
                if delta == 0 {
                    self.timer = 7;
                    self.sprite += 2;
                    self.state = 3;
                } else {
                    self.turn_timer = self.turn_timer.wrapping_sub(1);
                    if self.turn_timer == 0 {
                        self.turn_timer = 7;
                        self.heading = if delta & 16 == 0 {
                            self.heading.wrapping_add(1)
                        } else {
                            self.heading.wrapping_sub(1)
                        } & 31;
                    }
                }
            }
            3 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer == 0 {
                    self.state = 1;
                    return Some(BossSpawn {
                        source_type: 72,
                        position: position.into(),
                        direction: self.direction,
                        variant: self.missile_lifetime,
                    });
                }
            }
            _ => unreachable!(),
        }
        None
    }
}

/// Moving final building, including the original three-part beam cadence.
#[derive(Clone, Debug)]
pub struct HeadquartersBuilding {
    pub position: Vec2,
    pub state: u8,
    pub timer: u8,
    pub sprite: u16,
    pub damage_phase: u8,
    pub graphics_phase: Option<u8>,
    pub previous_graphics_phase: Option<u8>,
    pub previous_previous_graphics_phase: Option<u8>,
    pub ticks_since_graphics: u16,
    graphics_cursor: u8,
    velocity: f32,
    beam_count: u8,
}
impl HeadquartersBuilding {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            state: 0,
            timer: 0,
            sprite: 0,
            damage_phase: 0,
            graphics_phase: None,
            previous_graphics_phase: None,
            previous_previous_graphics_phase: None,
            ticks_since_graphics: 0,
            graphics_cursor: 0,
            velocity: 0.,
            beam_count: 0,
        }
    }
    fn move_and_check(&mut self, camera_x: f32) -> bool {
        self.position.x += self.velocity;
        let x = self.position.x - camera_x;
        (x < 48. && self.velocity <= 0.) || (x >= 208. && self.velocity >= 0.)
    }
    pub fn tick(
        &mut self,
        health: u16,
        clock: u8,
        player_x: [u8; 2],
        camera_x: f32,
    ) -> Option<BossSpawn> {
        if self.graphics_phase.is_some() {
            self.ticks_since_graphics = self.ticks_since_graphics.saturating_add(1);
        }
        if self.state != 0
            && self.damage_phase < 2
            && health < [42, 21][usize::from(self.damage_phase)]
        {
            self.damage_phase += 1;
        }
        match self.state {
            0 => {
                self.sprite = 0xd3;
                self.state = 1;
                self.move_and_check(camera_x);
                return Some(BossSpawn {
                    source_type: 79,
                    position: self.position.into(),
                    direction: 0,
                    variant: 0,
                });
            }
            1 => {
                // 9D47 leaves carry set when the current health threshold is met.
                let mut carry = true;
                let mut duration = adc(clock, player_x[0], &mut carry);
                duration = adc(duration, player_x[1], &mut carry) & 127;
                if duration < 32 {
                    duration |= 32;
                }
                self.timer = duration;
                if self.move_and_check(camera_x) {
                    self.timer = 112;
                }
                self.velocity = if self.position.x - camera_x >= 128. {
                    -1.25
                } else {
                    1.25
                };
                self.state = 2;
            }
            2 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer != 0
                    && (self.graphics_phase.is_none() || self.ticks_since_graphics >= 4)
                {
                    self.previous_previous_graphics_phase = self.previous_graphics_phase;
                    self.previous_graphics_phase = self.graphics_phase;
                    self.graphics_phase = Some(self.graphics_cursor);
                    self.ticks_since_graphics = 0;
                    self.graphics_cursor = if self.velocity < 0. {
                        self.graphics_cursor.wrapping_add(1)
                    } else {
                        self.graphics_cursor.wrapping_sub(1)
                    } & 3;
                }

                if self.move_and_check(camera_x) {
                    self.velocity = 0.;
                    self.state = 1;
                } else if self.timer == 0 {
                    self.velocity = 0.;
                    self.beam_count = 0;
                    self.state = 3;
                    return Some(BossSpawn {
                        source_type: 76,
                        position: [self.position.x - 4., self.position.y + 46.],
                        direction: 0,
                        variant: 0,
                    });
                }
            }
            3 => {
                if self.move_and_check(camera_x) {
                    self.velocity = 0.;
                    self.state = 1;
                } else {
                    if self.timer & 15 == 0 {
                        self.beam_count += 1;
                        if self.beam_count == 3 {
                            self.velocity = 0.;
                            self.state = 1;
                            return None;
                        }
                        self.timer = self.timer.wrapping_add(1);
                        return Some(BossSpawn {
                            source_type: 77,
                            position: [self.position.x - 4., self.position.y + 46.],
                            direction: 0,
                            variant: 0,
                        });
                    }
                    self.timer = self.timer.wrapping_add(1);
                }
            }
            _ => unreachable!(),
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct HeadquartersGun {
    pub position: Vec2,
    pub state: u8,
    pub sprite: u16,
    pub flips: (bool, bool),
    pub direction: u8,
    pub timer: u8,
    pub turn_timer: u8,
    heading: u8,
    orientation: u8,
    recoil_timer: u8,
    velocity: Vec2,
}
impl HeadquartersGun {
    pub fn new(camera_y: f32) -> Self {
        Self {
            position: Vec2::new(0., camera_y + 32.),
            state: 0,
            sprite: 0,
            flips: (false, false),
            direction: 0,
            timer: 0,
            turn_timer: 2,
            heading: 8,
            orientation: 0,
            recoil_timer: 0,
            velocity: Vec2::ZERO,
        }
    }
    fn draw(&mut self) {
        self.orientation = ((self.heading + 1) & 31) / 4;
        let entry = [0_u8, 1, 2, 0x41, 0x40, 0xc1, 0x82, 0x81][usize::from(self.orientation)];
        self.sprite = 0xfc + u16::from(entry & 15);
        self.flips = (entry & 64 != 0, entry & 128 != 0);
    }
    fn advance(&mut self, building_x: f32) {
        self.position.x =
            building_x.floor() + 7. + self.position.x.rem_euclid(1.) + self.velocity.x;
        self.position.y += self.velocity.y;
    }
    fn muzzle(&self) -> Vec2 {
        let i = usize::from(self.orientation);
        self.position
            + Vec2::new(
                [20., 12., 0., -12., -20., -12., 0., 12.][i],
                [0., 12., 20., 12., 0., -12., -20., -12.][i],
            )
    }
    pub fn tick(&mut self, building_x: f32, target: Vec2) -> Option<crate::boss::BossShot> {
        self.tick_with_building(building_x, target, true)
    }
    pub fn tick_with_building(
        &mut self,
        building_x: f32,
        target: Vec2,
        building_active: bool,
    ) -> Option<crate::boss::BossShot> {
        match self.state {
            0 => {
                self.draw();
                self.advance(building_x);
                self.state = 1;
            }
            1 => {
                self.draw();
                self.advance(building_x);
                let direction = crate::enemy::aim_direction(target - self.position);
                let delta = direction.wrapping_sub(self.heading);
                if delta == 0 {
                    self.timer = 64;
                    self.direction = crate::enemy::aim_direction(target - self.muzzle());
                    self.state = 2;
                } else {
                    self.turn_timer = self.turn_timer.wrapping_sub(1);
                    if self.turn_timer == 0 {
                        self.turn_timer = 2;
                        self.heading = if delta & 16 == 0 {
                            self.heading.wrapping_add(1)
                        } else {
                            self.heading.wrapping_sub(1)
                        } & 31;
                    }
                }
            }
            2 => {
                self.advance(building_x);
                self.timer = self.timer.wrapping_sub(1);
                if self.timer == 0 {
                    self.state = 1;
                } else if self.timer & 15 == 0 {
                    let shot = crate::boss::BossShot {
                        configuration: 11,
                        position: self.muzzle().into(),
                        velocity: crate::enemy::direction_velocity(self.direction, 2.25).into(),
                        radius: 3.,
                        lifetime: 36,
                        spawn_delay: 1,
                        sprite: 16,
                    };
                    self.recoil_timer = 3;
                    self.state = 3;
                    let i = usize::from(self.orientation & 3);
                    self.velocity = Vec2::new(
                        [-0.375, -0.375, 0., 0.375][i],
                        [0., -0.375, -0.375, -0.375][i],
                    );
                    if self.orientation & 4 != 0 {
                        self.velocity = self.velocity * -1.;
                    }
                    return Some(shot);
                }
            }
            3 => {
                self.advance(building_x);
                self.recoil_timer -= 1;
                if self.recoil_timer == 0 {
                    self.recoil_timer = 3;
                    self.velocity = self.velocity * -1.;
                    self.state = 4;
                }
            }
            4 => {
                self.recoil_timer -= 1;
                self.advance(building_x);
                if self.recoil_timer == 0 {
                    self.velocity = Vec2::ZERO;
                    self.state = if building_active { 2 } else { 5 };
                }
            }
            _ => unreachable!(),
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headquarters_gun_matches_original_700_tick_capture() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/hq_sequences.json")).unwrap();
        let capture = captures
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["source_type"] == 79)
            .unwrap();
        let mut gun = HeadquartersGun::new(0.);
        for sample in capture["samples"].as_array().unwrap() {
            let shot = gun.tick(0., Vec2::new(80., 160.));
            for (key, value) in [
                ("state", u64::from(gun.state)),
                ("sprite", u64::from(gun.sprite)),
                ("timer", u64::from(gun.timer)),
                ("fire_timer", u64::from(gun.turn_timer)),
                ("direction", u64::from(gun.direction)),
            ] {
                assert_eq!(
                    value,
                    sample[key].as_u64().unwrap(),
                    "tick{} {key}",
                    sample["tick"]
                );
            }
            assert_eq!(
                gun.position.x,
                sample["x"].as_f64().unwrap() as f32,
                "tick{} x",
                sample["tick"]
            );
            assert_eq!(
                gun.position.y,
                sample["y"].as_f64().unwrap() as f32,
                "tick{} y",
                sample["tick"]
            );
            assert_eq!(
                usize::from(shot.is_some()),
                sample["shots"].as_array().unwrap().len(),
                "tick{} shots",
                sample["tick"]
            );
        }
    }
    #[test]
    fn headquarters_building_matches_original_700_tick_capture() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/hq_sequences.json")).unwrap();
        let capture = captures
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["source_type"] == 75)
            .unwrap();
        let mut building = HeadquartersBuilding::new(Vec2::new(80., 80.));
        for sample in capture["samples"].as_array().unwrap() {
            let spawn = building.tick(127, 0, [80, 0], 0.);
            for (key, value) in [
                ("state", u64::from(building.state)),
                ("sprite", u64::from(building.sprite)),
                ("timer", u64::from(building.timer)),
            ] {
                assert_eq!(
                    value,
                    sample[key].as_u64().unwrap(),
                    "tick{} {key}",
                    sample["tick"]
                );
            }
            assert_eq!(
                building.position.x,
                sample["x"].as_f64().unwrap() as f32,
                "tick{} x",
                sample["tick"]
            );
            let shots: Vec<_> = spawn
                .into_iter()
                .map(|s| u64::from(s.source_type))
                .collect();
            let expected: Vec<_> = sample["shots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_u64().unwrap())
                .collect();
            assert_eq!(shots, expected, "tick{} shots", sample["tick"]);
        }
    }
    #[test]
    fn headquarters_actors_match_original_700_tick_capture() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/hq_sequences.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let typ = capture["source_type"].as_u64().unwrap();
            if typ == 75 || typ == 79 {
                continue;
            }
            let mut center = HeadquartersCenter::default();
            let mut guard = HeadquartersGuard::default();
            for sample in capture["samples"].as_array().unwrap() {
                let (state, sprite, timer, fire_timer, direction, flips, spawn) = if typ == 74 {
                    let spawn = center.tick(Vec2::new(80., 80.), 63, 0, [160, 0], 0.);
                    (
                        center.state,
                        center.sprite,
                        0,
                        center.fire_timer,
                        0,
                        0,
                        spawn,
                    )
                } else {
                    let spawn = guard.tick(Vec2::new(80., 80.), 0, [80, 0]);
                    (
                        guard.state,
                        guard.sprite,
                        guard.timer,
                        guard.turn_timer,
                        guard.direction,
                        u8::from(guard.flips.0) * 64 + u8::from(guard.flips.1) * 128,
                        spawn,
                    )
                };
                for (key, value) in [
                    ("state", u64::from(state)),
                    ("sprite", u64::from(sprite)),
                    ("timer", u64::from(timer)),
                    ("fire_timer", u64::from(fire_timer)),
                    ("direction", u64::from(direction)),
                    ("flips", u64::from(flips)),
                ] {
                    assert_eq!(
                        value,
                        sample[key].as_u64().unwrap(),
                        "type{typ} tick{} {key}",
                        sample["tick"]
                    );
                }
                let shots: Vec<_> = spawn
                    .into_iter()
                    .map(|s| u64::from(s.source_type))
                    .collect();
                let expected: Vec<_> = sample["shots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_u64().unwrap())
                    .collect();
                assert_eq!(shots, expected, "type{typ} tick{} shots", sample["tick"]);
            }
        }
    }
}
