//! Formation aircraft and cycling environmental hazards, recovered from bank 6.
use crate::math::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct EnvironmentSpawn {
    pub source_type: u8,
    pub position: Vec2,
    pub variant: u8,
    pub palette: u8,
}
#[derive(Default)]
pub struct EnvironmentEvents {
    pub spawns: Vec<EnvironmentSpawn>,
    pub shots: Vec<(u8, u8)>,
    pub sounds: Vec<u8>,
    pub art_command: Option<u8>,
}
pub struct EnvironmentContext {
    pub camera: Vec2,
    pub tick: u64,
    pub page: u8,
    pub forced_scroll: bool,
    pub target: Vec2,
    pub free_slots: usize,
}
pub struct EnvironmentActor {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub source_type: u8,
    pub position: Vec2,
    pub sprite: usize,
    pub palette: u8,
    pub alive: bool,
    pub harmful: bool,
    pub health: u16,
    phase: u8,
    variant: u8,
    timer: u8,
    animation: u8,
    half_clock: u8,
    shot_mask: u8,
    cycle: u8,
    sprite_base: usize,
    offset: f32,
}
impl EnvironmentActor {
    pub fn new(source_type: u8, position: Vec2, variant: u8, palette: u8) -> Option<Self> {
        if ![0x32, 0x33, 0x38, 0x0d, 0x49].contains(&source_type) {
            return None;
        }
        Some(Self {
            actor_handle: None,
            source_type,
            position,
            sprite: 0,
            palette,
            alive: true,
            harmful: false,
            health: if source_type == 0x33 { 4 } else { 0 },
            phase: 0,
            variant,
            timer: 0,
            animation: 0,
            half_clock: 0,
            shot_mask: 255,
            cycle: 0,
            sprite_base: 0,
            offset: 0.,
        })
    }
    pub fn kill(&mut self) {
        if self.source_type == 0x33 && self.phase < 2 {
            self.phase = 2;
            self.harmful = false;
        }
    }
    pub fn damage(&mut self, amount: u16) -> bool {
        if !self.vulnerable() {
            return false;
        }
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 {
            self.kill();
            return true;
        }
        false
    }
    pub fn vulnerable(&self) -> bool {
        self.source_type == 0x33 && self.phase < 2 && self.alive
    }
    fn visible(&self, c: &EnvironmentContext) -> bool {
        let p = self.position - c.camera;
        p.x >= 0. && p.x < 256. && p.y >= 0. && p.y < 256.
    }
    fn advance(&mut self, c: &EnvironmentContext, velocity: f32) {
        if !c.forced_scroll {
            self.position.y += velocity;
        }
        let y = self.position.y - c.camera.y;
        if self.position.x < 0. || self.position.x >= 512. || !(-64. ..320.).contains(&y) {
            self.alive = false;
            self.sprite = 0;
        }
    }
    pub fn tick(&mut self, c: &EnvironmentContext, formation_mask: &mut u8) -> EnvironmentEvents {
        let mut out = EnvironmentEvents::default();
        if !self.alive {
            return out;
        }
        match self.source_type {
            0x32 => match self.phase {
                0 => {
                    self.variant = 128;
                    *formation_mask = 255;
                    self.timer = 24;
                    self.phase = 1;
                }
                1 => {
                    self.advance(c, 0.);
                    if c.free_slots > 0 {
                        out.spawns.push(EnvironmentSpawn {
                            source_type: 0x33,
                            position: self.position,
                            variant: self.variant,
                            palette: 2,
                        });
                        self.variant >>= 1;
                        self.phase = 2;
                    }
                }
                2 => {
                    self.advance(c, 0.);
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer == 0 {
                        self.timer = 24;
                        if c.free_slots == 0 {
                            self.phase = 3;
                        } else {
                            out.spawns.push(EnvironmentSpawn {
                                source_type: 0x33,
                                position: self.position,
                                variant: self.variant,
                                palette: 1,
                            });
                            self.variant >>= 1;
                            if self.variant == 0 {
                                self.phase = 3;
                            }
                        }
                    }
                }
                _ => self.alive = false,
            },
            0x33 => match self.phase {
                0 => {
                    self.sprite = if self.variant & 128 != 0 { 165 } else { 168 };
                    self.shot_mask = if self.variant & 0x55 != 0 { 31 } else { 255 };
                    self.animation = 0;
                    self.half_clock = 0;
                    self.phase = 1;
                    self.harmful = true;
                }
                1 => {
                    self.animation = self.animation.wrapping_add(1);
                    if self.animation & 1 != 0 {
                        self.half_clock = self.half_clock.wrapping_add(1);
                        if matches!(self.half_clock, 115 | 120) {
                            self.sprite += 1;
                        }
                    }
                    self.advance(c, -1.5);
                    if !self.alive {
                        return out;
                    }
                    if self.half_clock == 128 {
                        self.alive = false;
                        self.sprite = 0;
                        return out;
                    }
                    if self.half_clock & self.shot_mask == 0 && self.visible(c) {
                        out.shots
                            .push((if c.target.y < self.position.y { 0 } else { 16 }, 6));
                    }
                    if *formation_mask & self.variant == 0 {
                        self.phase = 2;
                    }
                }
                2 => {
                    self.advance(c, 0.);
                    self.sprite = 25;
                    self.palette = 3;
                    self.timer = 24;
                    self.harmful = false;
                    self.phase = 3;
                    if self.visible(c) {
                        out.sounds.push(0x17);
                    }
                }
                _ => {
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer != 0 && self.timer & 7 == 0 {
                        self.sprite += 1;
                    }
                    self.advance(c, 0.);
                    if self.timer == 0 {
                        *formation_mask = !(self.variant >> 1);
                        self.alive = false;
                        self.sprite = 0;
                    }
                }
            },
            0x38 => match self.phase {
                0 => {
                    self.cycle = (c.tick & 3) as u8;
                    if self.cycle == 3 {
                        self.cycle = 0;
                    }
                    self.sprite_base = if c.page == 7 { 95 } else { 96 };
                    self.offset = if c.page == 7 { 64. } else { 56. };
                    self.timer = 16;
                    self.phase = 1;
                }
                1 => {
                    self.position.x = [240., 304., 368.][usize::from(self.cycle)];
                    self.cycle = (self.cycle + 1) % 3;
                    self.advance(c, 0.);
                    if self.visible(c) {
                        for dy in [self.offset, -self.offset].into_iter().take(c.free_slots) {
                            out.spawns.push(EnvironmentSpawn {
                                source_type: 0x0d,
                                position: Vec2::new(self.position.x, self.position.y + dy),
                                variant: 0,
                                palette: 3,
                            });
                        }
                        self.timer = 32;
                        self.phase = 2;
                    }
                }
                2 => {
                    self.timer -= 1;
                    if self.timer == 0 {
                        self.sprite = self.sprite_base;
                        self.harmful = true;
                    }
                    self.advance(c, 0.);
                    if self.timer == 0 {
                        self.timer = 24;
                        out.sounds.push(0x0b);
                        self.phase = 3;
                    }
                }
                _ => {
                    self.palette = (self.palette + 1) & 3;
                    self.timer -= 1;
                    if self.timer == 0 {
                        self.sprite = 0;
                        self.harmful = false;
                    }
                    self.advance(c, 0.);
                    if self.timer == 0 {
                        self.timer = 16;
                        self.phase = 1;
                    }
                }
            },
            0x0d => match self.phase {
                0 => {
                    self.sprite_base = 97;
                    self.timer = 16;
                    self.phase = 1;
                }
                1 => {
                    self.sprite = self.sprite_base;
                    self.timer -= 1;
                    if self.timer & 1 != 0 {
                        self.sprite = 0;
                    }
                    self.advance(c, 0.);
                    if self.timer == 0 {
                        self.phase = 2;
                    } else if self.timer == 8 {
                        self.sprite_base += 1;
                    }
                }
                _ => {
                    self.alive = false;
                    self.sprite = 0;
                }
            },
            0x49 => {
                if self.phase == 0 {
                    out.art_command = Some(0x10);
                    self.phase = 1;
                } else {
                    self.alive = false;
                }
            }
            _ => unreachable!(),
        }
        out
    }
}
