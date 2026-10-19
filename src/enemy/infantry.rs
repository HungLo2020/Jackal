//! Roaming infantry state machines, bank6 8263 and A470 (marsh variant).
use super::{aim_direction, direction_velocity, Enemy};
use crate::{math::Vec2, terrain::CollisionField, weapon::Projectile};
#[derive(Clone, Copy, Debug)]
enum Phase {
    Initialize,
    Waypoint,
    Turn,
    Walk,
    Approach,
    Pause,
    Fire,
}
#[derive(Clone, Debug)]
pub(super) struct Infantry {
    phase: Phase,
    heading: u8,
    waypoint_index: u8,
    rotation: i8,
    waypoint: Vec2,
    timer: u16,
    velocity: Vec2,
}
impl Default for Infantry {
    fn default() -> Self {
        Self {
            phase: Phase::Initialize,
            heading: 0,
            waypoint_index: 0,
            rotation: 1,
            waypoint: Vec2::new(0., 0.),
            timer: 0,
            velocity: Vec2::new(0., 0.),
        }
    }
}
impl Infantry {
    pub fn refreshes_target(&self) -> bool {
        matches!(self.phase, Phase::Pause) && self.timer == 1
    }
    pub(super) fn tick(
        &mut self,
        enemy: &mut Enemy,
        target: Vec2,
        terrain: &impl CollisionField,
    ) -> Vec<Projectile> {
        let marsh = enemy.definition == 4;
        let rank = enemy.rank;
        let terrain_at_actor = terrain.terrain_at(enemy.position);
        let base = if marsh && terrain_at_actor == 3 {
            116
        } else {
            18
        };
        let probe = terrain.terrain_at(lookahead(enemy.position, self.heading, marsh));
        let blocked = probe != 0 && !(marsh && probe == 3);
        match self.phase {
            Phase::Initialize => {
                enemy.current_sprite = Some(18);
                enemy.current_palette = Some(1);
                self.waypoint_index = (enemy.spawn_tick & 7) as u8;
                self.rotation = if self.waypoint_index & 1 == 0 { 1 } else { -1 };
                self.phase = Phase::Waypoint;
            }
            Phase::Waypoint => {
                enemy.position += self.velocity;
                self.waypoint_index =
                    (i16::from(self.waypoint_index) + i16::from(self.rotation)).rem_euclid(8) as u8;
                const OFF: [f32; 8] = [80., 57., 0., -57., -80., -57., 0., 57.];
                self.waypoint = target
                    + Vec2::new(
                        OFF[self.waypoint_index as usize],
                        OFF[((self.waypoint_index + 6) & 7) as usize],
                    );
                self.waypoint.x = self.waypoint.x.clamp(0., 511.);
                self.heading = aim_direction(self.waypoint - enemy.position);
                self.velocity = direction_velocity(self.heading, 0.25);
                self.timer = [256, 192, 128, 64, 32, 16][rank];
                self.phase = Phase::Turn;
            }
            Phase::Turn => {
                enemy.infantry_frame(base, self.heading);
                if blocked {
                    self.heading =
                        (i16::from(self.heading) + i16::from(self.rotation)).rem_euclid(32) as u8;
                    self.velocity = Vec2::new(0., 0.);
                }
                enemy.position += self.velocity;
                if blocked {
                    self.velocity = direction_velocity(self.heading, 0.25);
                } else {
                    self.phase = Phase::Walk;
                }
            }
            Phase::Walk | Phase::Approach => {
                enemy.infantry_frame(base + if self.timer & 8 != 0 { 3 } else { 0 }, self.heading);
                self.velocity = direction_velocity(
                    self.heading,
                    if marsh && probe == 3 { 0.125 } else { 0.25 },
                );
                if blocked {
                    self.velocity = Vec2::new(0., 0.);
                }
                enemy.position += self.velocity;
                self.timer -= 1;
                if self.timer == 0 {
                    self.next_walk_phase();
                } else if blocked {
                    self.velocity = direction_velocity(self.heading, 0.25);
                    self.phase = Phase::Turn;
                } else if self.timer >= 2 {
                    if enemy.position.x.floor() == self.waypoint.x.floor() {
                        if self.heading & 15 == 0 {
                            self.next_walk_phase();
                            return Vec::new();
                        }
                        self.heading = (self.heading & 16) | 8;
                    }
                    if enemy.position.y.floor() == self.waypoint.y.floor() {
                        if self.heading.wrapping_add(8) & 15 == 0 {
                            self.next_walk_phase();
                            return Vec::new();
                        }
                        self.heading = self.heading.wrapping_add(8) & 16;
                    }
                }
            }
            Phase::Pause => {
                self.heading = aim_direction(target - enemy.position);
                enemy.infantry_frame(base, self.heading);
                enemy.position += self.velocity;
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = [8, 8, 16, 9, 24, 17][rank];
                    let d = target - enemy.position;
                    if d.x.abs() + d.y.abs() >= [80., 64., 48., 32., 16., 0.][rank] {
                        self.phase = Phase::Fire;
                    } else {
                        self.reset(enemy);
                    }
                }
            }
            Phase::Fire => {
                self.heading = aim_direction(target - enemy.position);
                enemy.infantry_frame(base, self.heading);
                enemy.position += self.velocity;
                self.timer = self.timer.wrapping_sub(1);
                if self.timer.is_multiple_of(8) {
                    return vec![Projectile::enemy_pattern(
                        enemy.position,
                        direction_velocity(self.heading, 1.),
                        48 + rank as u16 * 8,
                        9,
                        16,
                    )];
                }
                if self.timer == u16::MAX - 6 {
                    self.reset(enemy);
                } else if self.timer <= 127 {
                    enemy.current_palette = enemy.current_palette.map(|p| (p + 1) & 3);
                }
            }
        }
        Vec::new()
    }
    fn next_walk_phase(&mut self) {
        if matches!(self.phase, Phase::Approach) {
            self.timer = 64;
            self.velocity = Vec2::new(0., 0.);
            self.phase = Phase::Pause;
        } else {
            self.timer = 128;
            self.phase = Phase::Approach;
        }
    }
    fn reset(&mut self, enemy: &mut Enemy) {
        enemy.current_palette = Some(1);
        self.phase = Phase::Waypoint;
    }
}
/// Original 9FD7/A030 lookahead, including its shared fractional component.
pub(crate) fn lookahead(position: Vec2, direction: u8, wide: bool) -> Vec2 {
    const SMALL_HI: [i32; 16] = [0, 1, 3, 4, 5, 6, 7, 7, 8, 7, 7, 6, 5, 4, 3, 1];
    const SMALL_LO: [i32; 16] = [
        0, 143, 15, 113, 168, 166, 100, 216, 0, 216, 100, 166, 168, 113, 15, 143,
    ];
    const LARGE_HI: [i32; 16] = [0, 3, 6, 8, 11, 13, 14, 15, 16, 15, 14, 13, 11, 8, 6, 3];
    const LARGE_LO: [i32; 16] = [
        0, 31, 31, 227, 80, 77, 200, 177, 0, 177, 200, 77, 80, 227, 31, 31,
    ];
    let (hi, lo) = if wide {
        (LARGE_HI, LARGE_LO)
    } else {
        (SMALL_HI, SMALL_LO)
    };
    let dx = (direction + 8) & 31;
    let ix = (dx & 15) as usize;
    let iy = (direction & 15) as usize;
    let mut x = (hi[ix] << 8) | lo[ix];
    if dx & 16 != 0 {
        x = -x;
    }
    let mut y = (hi[iy] << 8) | (x & 255);
    if direction & 16 != 0 {
        y = -y;
    }
    position + Vec2::new(x as f32 / 256., y as f32 / 256.)
}
