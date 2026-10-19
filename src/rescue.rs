//! Prisoner states recovered from bank 6:9922–9AA9.
use crate::{enemy::direction_velocity, math::Vec2, terrain::CollisionField};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Initialize,
    Waiting,
    Emerging,
    Walking,
    Idle,
}
#[derive(Clone, Debug)]
pub struct Prisoner {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub id: u64,
    pub previous: Option<u64>,
    pub special: bool,
    pub sprite: u16,
    pub palette: u8,
    pub flip_x: bool,
    phase: Phase,
    timer: u8,
    flags: u8,
    direction: u8,
    velocity: Vec2,
    parity: bool,
    slot: u8,
}
impl Prisoner {
    /// A free prisoner, already available for collection (editor/test entry point).
    pub fn waiting(position: Vec2, special: bool) -> Self {
        Self {
            actor_handle: None,
            phase: Phase::Idle,
            timer: 99,
            sprite: 43,
            ..Self::released(position, u8::from(special), 0, 0)
        }
    }
    pub fn released(position: Vec2, flags: u8, direction: u8, slot: u8) -> Self {
        Self {
            actor_handle: None,
            position,
            id: 0,
            previous: None,
            special: flags & 1 != 0,
            sprite: 0,
            palette: 0,
            flip_x: false,
            phase: Phase::Initialize,
            timer: 0,
            flags,
            direction,
            velocity: Vec2::ZERO,
            parity: false,
            slot,
        }
    }
    pub fn set_actor_handle(&mut self, handle: crate::actor_pool::ActorHandle) {
        self.actor_handle = Some(handle);
        self.slot = handle.slot();
    }
    pub fn collectible(&self) -> bool {
        matches!(self.phase, Phase::Walking | Phase::Idle)
    }
    pub fn emerged(&self) -> bool {
        !matches!(self.phase, Phase::Initialize | Phase::Waiting)
    }
    fn frame(&mut self) {
        let octant = usize::from(((self.direction + 1) & 31) / 4);
        self.sprite = 37 + [0, 0, 1, 1, 0, 0, 2, 2][octant];
        self.flip_x = matches!(octant, 4 | 5);
    }
    fn walking(&mut self) {
        self.velocity = direction_velocity(self.direction, 0.5);
        self.timer = 128;
        if self.flags == 0 {
            self.velocity = Vec2::ZERO;
            self.timer = 1;
        }
        self.phase = Phase::Walking;
    }
    pub fn tick(&mut self, tick: u64, previous_present: bool, terrain: &impl CollisionField) {
        match self.phase {
            Phase::Initialize => {
                self.parity = tick & 1 != 0;
                self.phase = Phase::Waiting;
            }
            Phase::Waiting => {
                self.timer = self.timer.wrapping_sub(1);
                self.sprite = if self.timer & 8 == 0 { 44 } else { 43 };
                if self.flags == 0 && previous_present {
                    return;
                }
                if self.flags & 2 != 0 {
                    self.walking();
                } else {
                    self.timer = if self.special { 28 } else { 72 };
                    self.velocity =
                        direction_velocity(self.direction, if self.special { 1. } else { 0.5 });
                    self.phase = Phase::Emerging;
                }
            }
            Phase::Emerging => {
                if self.flags != 0 {
                    self.palette = (self.palette + 1) & 3;
                }
                self.frame();
                self.timer = self.timer.wrapping_sub(1);
                if self.timer & 8 == 0 {
                    self.sprite += 3;
                }
                self.position += self.velocity;
                if self.timer == 0 {
                    self.direction = tick as u8 & 31;
                    self.walking();
                }
            }
            Phase::Walking => {
                if self.special {
                    self.palette = (self.palette + 1) & 3;
                }
                self.frame();
                self.timer = self.timer.wrapping_sub(1);
                if self.timer & 8 != 0 {
                    self.sprite += 3;
                }
                if !terrain.traversable(
                    self.position + direction_velocity(self.direction, 8.),
                    false,
                ) {
                    self.velocity = Vec2::ZERO;
                }
                self.position += self.velocity;
                if self.timer == 0 {
                    self.timer = 99;
                    self.velocity = Vec2::ZERO;
                    self.phase = Phase::Idle;
                }
            }
            Phase::Idle => {
                if self.special {
                    self.palette = (self.palette + 1) & 3;
                }
                self.timer = self.timer.wrapping_sub(1);
                self.sprite = if self.timer & 8 == 0 { 44 } else { 43 };
                if self.timer == 0 {
                    self.timer = 128;
                    self.direction = if self.parity {
                        self.slot.wrapping_sub(tick as u8)
                    } else {
                        self.slot.wrapping_add(tick as u8)
                    } & 31;
                    if self.flags != 0 {
                        self.velocity = direction_velocity(self.direction, 0.5);
                        self.phase = Phase::Walking;
                    }
                }
            }
        }
    }
}

/// A rescued passenger walking from the jeep to the helicopter (9AAC).
#[derive(Clone, Debug)]
pub struct Delivery {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub owner: usize,
    pub remaining: u8,
    pub last: bool,
    pub palette: u8,
    pub left: bool,
}
impl Delivery {
    pub fn tick(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        self.position.x += if self.left { -0.5 } else { 0.5 };
        if self.last {
            self.palette = (self.palette + 1) & 3;
        }
        self.remaining == 0
    }
    pub fn sprite(&self) -> u16 {
        37 + if self.remaining & 8 != 0 { 3 } else { 0 }
    }
}

/// The destroyed building keeps its smoke and queues linked prisoners (9848).
#[derive(Clone, Debug)]
pub struct PrisonRelease {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub source_type: u8,
    pub age: u16,
    remaining: u8,
    pub previous: Option<u64>,
}
impl PrisonRelease {
    pub fn new(position: Vec2, source_type: u8) -> Self {
        Self {
            actor_handle: None,
            position,
            source_type,
            age: 0,
            remaining: match source_type {
                19 | 20 => 2,
                28 | 29 => 4,
                21 => 1,
                _ => 0,
            },
            previous: None,
        }
    }
    pub fn sprite(&self) -> Option<(u16, u8)> {
        if self.age < 24 {
            Some((25 + self.age / 8, 3))
        } else if matches!(self.source_type, 19 | 20 | 28 | 29)
            && (25..89).contains(&self.age)
            && (89 - self.age) & 8 != 0
        {
            Some((45, 0))
        } else {
            None
        }
    }
    pub fn tick(&mut self, previous_emerged: bool) -> bool {
        self.age += 1;
        let ready = if self.source_type == 21 { 25 } else { 90 };
        if self.remaining > 0 && self.age >= ready && previous_emerged {
            self.remaining -= 1;
            true
        } else {
            false
        }
    }
    pub fn finished(&self) -> bool {
        self.remaining == 0 && self.age >= 90
    }
    pub fn prisoner(&self, id: u64) -> Prisoner {
        let mut p = Prisoner::released(
            self.position,
            u8::from(self.source_type == 21),
            match self.source_type {
                20 | 29 => 16,
                21 => 8,
                _ => 0,
            },
            (id % 16) as u8,
        );
        p.id = id;
        p.previous = self.previous;
        p
    }
}

/// Survivors scatter one per tick after jeep destruction (9B52).
#[derive(Clone, Debug)]
pub struct PassengerDrop {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    remaining: u8,
}
impl PassengerDrop {
    pub fn new(position: Vec2, passengers: u8) -> Self {
        Self {
            actor_handle: None,
            position,
            remaining: passengers.saturating_sub(1).min(4),
        }
    }
    pub fn next(&mut self, tick: u64, id: u64) -> Option<Prisoner> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let direction = (self.remaining * 8).wrapping_add(tick as u8) & 31;
        let mut p = Prisoner::released(
            self.position,
            if direction == 0 { 3 } else { 2 },
            direction,
            (id % 16) as u8,
        );
        p.id = id;
        Some(p)
    }
    pub fn finished(&self) -> bool {
        self.remaining == 0
    }
}
