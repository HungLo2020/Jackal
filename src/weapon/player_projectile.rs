//! Player weapon states recovered from ED2C..F07F. Timers include initialization,
//! flight, impact, and cleanup frames; these states also reserve the weapon slot.
use super::{Projectile, ProjectileKind};
use crate::{
    math::Vec2,
    player::{Facing, WeaponLevel},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Initialize,
    Flight,
    PendingBlast,
    Blast,
    Impact,
    WaveInitialize,
    Wave,
    Remove,
    Done,
}
#[derive(Clone, Debug)]
pub(super) struct PlayerFlight {
    phase: Phase,
    timer: u16,
    frame: Option<u16>,
    flips: (bool, bool),
    palette: u8,
    origin: Vec2,
    vertical: bool,
}
impl Projectile {
    pub fn player(
        position: Vec2,
        velocity: Vec2,
        kind: ProjectileKind,
        owner: usize,
        lifetime: u16,
        upgrade: WeaponLevel,
    ) -> Self {
        let mut shot = Self::new(position, velocity, kind, Some(owner), lifetime, upgrade);
        if matches!(kind, ProjectileKind::Grenade | ProjectileKind::Rocket) {
            shot.player_slot = Some(24 + owner as u8);
        }
        shot.player_state = Some(PlayerFlight {
            phase: Phase::Initialize,
            timer: lifetime,
            frame: None,
            flips: (false, false),
            palette: 1,
            origin: position,
            vertical: false,
        });
        shot
    }
    pub fn blast_wave(position: Vec2, owner: usize, vertical: bool) -> Self {
        let mut shot = Self::new(
            position,
            Vec2::new(0., 0.),
            ProjectileKind::BlastWave,
            Some(owner),
            42,
            WeaponLevel::FourWay,
        );
        shot.player_slot = Some(if vertical { 28 } else { 26 } + owner as u8);
        shot.player_state = Some(PlayerFlight {
            phase: Phase::WaveInitialize,
            timer: 40,
            frame: None,
            flips: (false, false),
            palette: 3,
            origin: position,
            vertical,
        });
        shot
    }
    pub fn blast_waves(&self) -> Vec<Self> {
        let Some(owner) = self.owner else {
            return Vec::new();
        };
        let mut waves = Vec::new();
        if matches!(self.upgrade, WeaponLevel::TwoWay | WeaponLevel::FourWay) {
            waves.push(Self::blast_wave(self.position, owner, false));
        }
        if self.upgrade == WeaponLevel::FourWay {
            waves.push(Self::blast_wave(self.position, owner, true));
        }
        waves
    }
    /// The final headquarters encounter extends the secondary travel timer.
    pub fn set_flight_ticks(&mut self, ticks: u16) {
        if let Some(s) = &mut self.player_state {
            if s.phase == Phase::Initialize {
                s.timer = ticks;
                self.remaining = ticks;
            }
        }
    }
    pub fn has_native_player_state(&self) -> bool {
        self.player_state.is_some()
    }
    pub fn is_player_blast(&self) -> bool {
        self.player_state
            .as_ref()
            .is_some_and(|s| s.phase == Phase::Blast)
    }
    pub fn is_flying(&self) -> bool {
        self.player_state
            .as_ref()
            .is_none_or(|s| s.phase == Phase::Flight)
    }
    pub fn can_hit_enemies(&self) -> bool {
        self.player_state
            .as_ref()
            .is_none_or(|s| !matches!(s.phase, Phase::Impact | Phase::Done | Phase::Initialize))
    }
    pub fn player_sprite(&self) -> Option<u16> {
        self.player_state.as_ref().and_then(|s| s.frame)
    }
    pub fn player_palette(&self) -> Option<u8> {
        self.player_state.as_ref().map(|s| s.palette)
    }
    pub(super) fn player_flips(&self) -> Option<(bool, bool)> {
        self.player_state.as_ref().map(|s| s.flips)
    }
    /// Player impacts select the next native state instead of deleting the shot.
    pub fn impact(&mut self) {
        if let Some(s) = &mut self.player_state {
            if !matches!(s.phase, Phase::Flight | Phase::Initialize) {
                return;
            }
            if self.kind == ProjectileKind::Bullet {
                s.phase = Phase::Impact;
                s.frame = Some(17);
                s.timer = 5;
                self.velocity = Vec2::new(0., 0.);
                self.remaining = 5;
            } else {
                s.phase = Phase::PendingBlast;
                self.remaining = 25;
            }
        } else {
            self.remaining = 0;
        }
    }
    pub fn tick_player(&mut self, global_tick: u64) -> bool {
        let Some(s) = &mut self.player_state else {
            self.tick();
            return false;
        };
        self.previous = self.position;
        self.age += 1;
        let mut detonated = false;
        match s.phase {
            Phase::Initialize => {
                s.frame = Some(match self.kind {
                    ProjectileKind::Bullet => 16,
                    ProjectileKind::Grenade => 7,
                    _ => {
                        let facing = Facing::from_direction(
                            if self.velocity.x == 0. {
                                0
                            } else {
                                self.velocity.x.signum() as i8
                            },
                            if self.velocity.y == 0. {
                                0
                            } else {
                                self.velocity.y.signum() as i8
                            },
                        )
                        .unwrap_or(Facing::North) as usize;
                        let attrs = [0u8, 0, 0, 0x80, 0x80, 0xc0, 0x40, 0x40][facing];
                        s.flips = (attrs & 64 != 0, attrs & 128 != 0);
                        [13, 14, 15, 14, 13, 14, 15, 14][facing]
                    }
                });
                s.palette = if self.kind == ProjectileKind::Grenade {
                    0
                } else {
                    1
                };
                s.phase = Phase::Flight;
            }
            Phase::Flight => {
                if self.kind == ProjectileKind::Grenade {
                    s.frame = Some(if (14..26).contains(&s.timer) { 8 } else { 7 });
                    let attrs = [0u8, 128, 64, 192][((global_tick & 15) / 4) as usize];
                    s.flips = (attrs & 64 != 0, attrs & 128 != 0);
                }
                s.timer = s.timer.saturating_sub(1);
                if self.kind == ProjectileKind::Bullet && s.timer == 0 {
                    s.phase = Phase::Impact;
                    s.timer = 5;
                    s.frame = Some(17);
                    self.velocity = Vec2::new(0., 0.);
                } else {
                    self.position += self.velocity;
                    if s.timer == 0 {
                        s.phase = Phase::PendingBlast;
                    }
                }
            }
            Phase::PendingBlast => {
                s.phase = Phase::Blast;
                s.frame = Some(25);
                s.palette = 3;
                s.timer = 8;
                self.velocity = Vec2::new(0., 0.);
                detonated = true;
            }
            Phase::Blast => {
                s.timer -= 1;
                if s.timer == 0 {
                    let frame = s.frame.unwrap() + 1;
                    s.timer = 8;
                    if frame == 28 {
                        s.phase = Phase::Done;
                        s.frame = None;
                    } else {
                        s.frame = Some(frame);
                    }
                }
            }
            Phase::Impact => {
                s.timer -= 1;
                if s.timer == 0 {
                    s.phase = Phase::Done;
                    s.frame = None;
                }
            }
            Phase::WaveInitialize => {
                s.phase = Phase::Wave;
                s.frame = Some(9);
            }
            Phase::Wave => {
                if s.timer < 24 {
                    s.frame = Some(if s.timer < 8 { 11 } else { 10 });
                }
                if global_tick.is_multiple_of(4) {
                    s.flips.1 = !s.flips.1;
                }
                let step = 41 - s.timer;
                let offset = f32::from(2 + step * 2) * if step % 2 == 1 { 1. } else { -1. };
                self.position = s.origin
                    + if s.vertical {
                        Vec2::new(0., offset)
                    } else {
                        Vec2::new(offset, 0.)
                    };
                s.timer -= 1;
                if s.timer == 0 {
                    s.phase = Phase::Remove;
                }
            }
            Phase::Remove => {
                s.phase = Phase::Done;
                s.frame = None;
            }
            Phase::Done => {}
        }
        self.remaining = match s.phase {
            Phase::Done => 0,
            Phase::Flight | Phase::Impact => s.timer.max(1),
            Phase::Wave | Phase::Remove => s.timer + 1,
            _ => 1,
        };
        detonated
    }
}
