//! Rescue helicopter waiting, rotor spin-up, departure and return fly-through.
use crate::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelicopterKind {
    Flyover,
    LandingLeft,
    LandingRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelicopterPhase {
    Initializing,
    Waiting,
    BoardingGrace,
    Spooling,
    Departing,
    ReturnDelay,
    Flyover,
    Gone,
}
pub struct HelicopterContext {
    pub camera: Vec2,
    pub tick: u64,
    pub carried_prisoners: u16,
    pub forced_scroll: bool,
}
pub struct RescueHelicopter {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub kind: HelicopterKind,
    pub position: Vec2,
    pub phase: HelicopterPhase,
    pub sprite: usize,
    pub flip_y: bool,
    pub dock_open: bool,
    animation_tick: u8,
    wait: u8,
    grace: u8,
    spool: u8,
    sound_mask: u8,
    rotor_base: usize,
    return_wait: u8,
    velocity_y: f32,
}
impl RescueHelicopter {
    pub fn new(source_type: u8, position: Vec2) -> Option<Self> {
        let kind = match source_type {
            0x3c => HelicopterKind::Flyover,
            0x3d => HelicopterKind::LandingLeft,
            0x3e => HelicopterKind::LandingRight,
            _ => return None,
        };
        Some(Self {
            actor_handle: None,
            kind,
            position,
            phase: HelicopterPhase::Initializing,
            sprite: 0,
            flip_y: false,
            dock_open: false,
            animation_tick: 0,
            wait: 128,
            grace: 0,
            spool: 64,
            sound_mask: 255,
            rotor_base: 47,
            return_wait: 128,
            velocity_y: 0.,
        })
    }
    pub fn unloading_direction(&self) -> Option<i8> {
        if !self.dock_open {
            return None;
        }
        match self.kind {
            HelicopterKind::LandingLeft => Some(1),
            HelicopterKind::LandingRight => Some(-1),
            HelicopterKind::Flyover => None,
        }
    }
    pub fn alive(&self) -> bool {
        self.phase != HelicopterPhase::Gone
    }
    fn passed_parking_area(&self, context: &HelicopterContext) -> bool {
        self.position.y - context.camera.y >= 304.
    }
    fn depart(&mut self) {
        self.velocity_y = -2.25;
        self.dock_open = false;
        self.phase = HelicopterPhase::Departing;
    }
    fn move_and_cull(&mut self, context: &HelicopterContext) {
        if !(context.forced_scroll && self.kind == HelicopterKind::Flyover) {
            self.position.y += self.velocity_y;
        }
        let y = self.position.y - context.camera.y;
        if self.position.x < 0. || self.position.x >= 512. || !(-64. ..320.).contains(&y) {
            self.phase = HelicopterPhase::Gone;
            self.sprite = 0;
            self.dock_open = false;
        }
    }
    fn parked_animation(&mut self) {
        self.animation_tick = self.animation_tick.wrapping_add(1);
        self.sprite = 47 + usize::from(self.animation_tick & 4 != 0);
    }
    /// Returns original sound command 0x0c on each rotor cue.
    pub fn tick(&mut self, context: &HelicopterContext) -> Vec<u8> {
        let mut sounds = Vec::new();
        match self.phase {
            HelicopterPhase::Initializing => {
                if self.kind == HelicopterKind::Flyover {
                    self.sprite = 49;
                    self.position.x = context.camera.x + 64. + (context.tick & 127) as f32;
                    self.velocity_y = -2.5;
                    self.phase = HelicopterPhase::Flyover;
                } else {
                    self.sprite = 47;
                    self.dock_open = true;
                    self.phase = HelicopterPhase::Waiting;
                }
            }
            HelicopterPhase::Waiting => {
                self.parked_animation();
                self.move_and_cull(context);
                if !self.alive() {
                    return sounds;
                }
                self.wait = self.wait.wrapping_sub(1);
                if self.wait >= 128 {
                    self.wait = self.wait.wrapping_add(1);
                    if context.carried_prisoners == 0 || self.passed_parking_area(context) {
                        self.phase = HelicopterPhase::BoardingGrace;
                    }
                }
            }
            HelicopterPhase::BoardingGrace => {
                self.parked_animation();
                self.move_and_cull(context);
                if !self.alive() {
                    return sounds;
                }
                if self.passed_parking_area(context) {
                    self.depart();
                } else {
                    self.grace = self.grace.wrapping_sub(1);
                    if self.grace == 0 {
                        self.phase = if context.carried_prisoners == 0 {
                            HelicopterPhase::Spooling
                        } else {
                            HelicopterPhase::Waiting
                        };
                    }
                }
            }
            HelicopterPhase::Spooling => {
                self.dock_open = false;
                if self.animation_tick & self.sound_mask == 0 {
                    sounds.push(0x0c);
                }
                self.animation_tick = self.animation_tick.wrapping_add(1);
                self.sprite = self.rotor_base + usize::from(self.animation_tick & 2 != 0);
                self.move_and_cull(context);
                if !self.alive() {
                    return sounds;
                }
                if self.animation_tick & 15 == 0 {
                    self.sound_mask >>= 1;
                }
                self.spool -= 1;
                if self.spool == 1 {
                    self.depart();
                } else {
                    if self.spool == 32 {
                        self.rotor_base = 49;
                    }
                    if self.passed_parking_area(context) {
                        self.depart();
                    }
                }
            }
            HelicopterPhase::Departing | HelicopterPhase::Flyover => {
                let flyover = self.phase == HelicopterPhase::Flyover;
                if self.animation_tick & 3 == 0 {
                    sounds.push(0x0c);
                }
                self.animation_tick = self.animation_tick.wrapping_add(1);
                self.sprite = 49 + usize::from(self.animation_tick & 2 != 0);
                self.move_and_cull(context);
                if !flyover && self.alive() && (self.position.y - context.camera.y) < 0. {
                    self.phase = HelicopterPhase::ReturnDelay;
                    self.sprite = 0;
                }
            }
            HelicopterPhase::ReturnDelay => {
                if self.animation_tick & 3 == 0 {
                    sounds.push(0x0c);
                }
                self.animation_tick = self.animation_tick.wrapping_add(1);
                self.return_wait -= 1;
                if self.return_wait == 0 {
                    self.position = Vec2::new(
                        context.camera.x + 64. + (context.tick & 127) as f32,
                        context.camera.y + self.position.y.rem_euclid(1.),
                    );
                    self.velocity_y = 2.5;
                    self.flip_y = true;
                    self.phase = HelicopterPhase::Departing;
                }
            }
            HelicopterPhase::Gone => {}
        }
        sounds
    }
}

/// Source3F remains a landing collision zone and admits timed reinforcements.
#[derive(Clone, Debug, Default)]
pub struct RescueZoneController {
    phase: u8,
    ticks: u16,
    attempts: u8,
}
impl RescueZoneController {
    pub fn alive(&self) -> bool {
        self.phase < 3
    }
    pub fn tick(
        &mut self,
        position: Vec2,
        camera: Vec2,
        stage: u8,
        dock_open: bool,
    ) -> Option<crate::environment::EnvironmentSpawn> {
        if self.phase == 0 {
            self.phase = 1;
            return None;
        }
        if self.phase >= 2 {
            self.phase = 3;
            return None;
        }
        let screen_y = position.y - camera.y;
        if !(0. ..512.).contains(&position.x) || !(-64. ..320.).contains(&screen_y) {
            self.phase = 3;
            return None;
        }
        self.ticks = self.ticks.wrapping_add(1);
        let mut spawn = None;
        if self.ticks & 511 == 0 && self.attempts != stage {
            self.attempts = self.attempts.wrapping_add(1);
            let source_type = [0, 59, 0, 0, 59, 67][usize::from(stage.min(5))];
            if source_type != 0 {
                spawn = Some(crate::environment::EnvironmentSpawn {
                    source_type,
                    position: Vec2::new(position.x - screen_y.floor(), camera.y),
                    variant: 0,
                    palette: [0, 1, 0, 0, 1, 2][usize::from(stage.min(5))],
                });
            }
        }
        if !dock_open {
            self.phase = 2;
        }
        spawn
    }
}
