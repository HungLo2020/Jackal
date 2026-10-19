//! Rotating tank turrets05/06: B7D8..BAA1, including muzzle placement and recoil.
use super::{aim_direction, direction_velocity, Enemy};
use crate::{math::Vec2, weapon::Projectile};
#[derive(Clone, Copy, Debug)]
enum Phase {
    Initialize,
    Aim,
    Attack,
    Recoil,
    Return,
}
#[derive(Clone, Debug)]
pub(super) struct RotatingTurret {
    phase: Phase,
    heading: u8,
    shot_heading: u8,
    octant: u8,
    turn_timer: u8,
    attack_timer: u8,
    recoil_timer: u8,
    velocity: Vec2,
}
impl Default for RotatingTurret {
    fn default() -> Self {
        Self {
            phase: Phase::Initialize,
            heading: 8,
            shot_heading: 8,
            octant: 2,
            turn_timer: 0,
            attack_timer: 0,
            recoil_timer: 0,
            velocity: Vec2::default(),
        }
    }
}
impl RotatingTurret {
    pub fn advances_target(&self) -> bool {
        matches!(self.phase, Phase::Aim)
    }
    fn frame(&self, e: &mut Enemy) {
        let attrs =
            [0u8, 1, 2, 0x41, 0x40, 0xc1, 0x82, 0x81][usize::from(((self.heading + 1) & 31) / 4)];
        e.current_sprite = Some(28 + u16::from(attrs & 15));
        e.sprite_flip_x = attrs & 64 != 0;
        e.sprite_flip_y = attrs & 128 != 0;
    }
    fn muzzle(&self) -> Vec2 {
        const X: [f32; 8] = [20., 12., 0., -12., -20., -12., 0., 12.];
        const Y: [f32; 8] = [0., 12., 20., 12., 0., -12., -20., -12.];
        Vec2::new(X[self.octant as usize], Y[self.octant as usize])
    }
    pub(super) fn tick(&mut self, e: &mut Enemy, target: Vec2) -> Vec<Projectile> {
        let turn_delay = 13u8.wrapping_sub(e.progression) / 2;
        match self.phase {
            Phase::Initialize => {
                self.frame(e);
                e.current_palette = Some(if e.stage == 5 {
                    if e.definition == 5 {
                        0
                    } else {
                        1
                    }
                } else {
                    2
                });
                self.turn_timer = turn_delay;
                self.phase = Phase::Aim;
            }
            Phase::Aim => {
                self.frame(e);
                self.octant = ((self.heading + 1) & 31) / 4;
                e.position += self.velocity;
                let wanted = aim_direction(target - e.position);
                let delta = wanted.wrapping_sub(self.heading);
                if delta == 0 {
                    self.attack_timer = [64, 56, 48, 40, 32, 24][e.rank];
                    self.shot_heading = if e.definition == 6 {
                        self.octant * 4
                    } else {
                        aim_direction(target - (e.position + self.muzzle()))
                    };
                    self.phase = Phase::Attack;
                } else {
                    self.turn_timer = self.turn_timer.wrapping_sub(1);
                    if self.turn_timer == 0 {
                        self.turn_timer = turn_delay;
                        let candidate = if delta & 16 == 0 {
                            self.heading.wrapping_add(1) & 31
                        } else {
                            self.heading.wrapping_sub(1) & 31
                        };
                        let octant = ((candidate + 1) & 31) / 4;
                        if e.definition == 5
                            || (delta & 16 == 0 && octant < 4)
                            || (delta & 16 != 0 && octant != 0)
                        {
                            self.heading = candidate;
                        }
                    }
                }
            }
            Phase::Attack => {
                e.position += self.velocity;
                self.attack_timer = self.attack_timer.wrapping_sub(1);
                if self.attack_timer == 0 {
                    self.phase = Phase::Aim;
                } else if self.attack_timer & 15 == 0 {
                    e.current_sprite = e.current_sprite.map(|f| f + 3);
                    self.recoil_timer = 3;
                    self.phase = Phase::Recoil;
                    let i = usize::from(self.octant & 3);
                    self.velocity = Vec2::new(
                        [-0.375, -0.375, 0., 0.375][i],
                        [0., -0.375, -0.375, -0.375][i],
                    );
                    if self.octant & 4 != 0 {
                        self.velocity = self.velocity * -1.;
                    }
                    return vec![Projectile::enemy_pattern(
                        e.position + self.muzzle(),
                        direction_velocity(
                            self.shot_heading,
                            if e.definition == 5 { 1. } else { 1.25 },
                        ),
                        96 + e.rank as u16 * 8,
                        1,
                        16,
                    )];
                }
            }
            Phase::Recoil => {
                e.position += self.velocity;
                self.recoil_timer -= 1;
                if self.recoil_timer == 0 {
                    self.recoil_timer = 3;
                    self.velocity = self.velocity * -1.;
                    self.phase = Phase::Return;
                }
            }
            Phase::Return => {
                self.recoil_timer -= 1;
                if self.recoil_timer == 0 {
                    e.current_sprite = e.current_sprite.map(|f| f - 3);
                }
                e.position += self.velocity;
                if self.recoil_timer == 0 {
                    self.velocity = Vec2::default();
                    self.phase = Phase::Attack;
                }
            }
        }
        Vec::new()
    }
}
