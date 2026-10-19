//! Ambush entrances and troop truck handlers8CF0..8E1E and922F..92AC.
use super::*;
#[derive(Clone, Debug, Default)]
pub(super) struct ScriptedVehicle {
    phase: u8,
    timer: u8,
    remaining: u8,
    velocity: Vec2,
    direction: u8,
}
impl ScriptedVehicle {
    pub fn terrain_patch(&self) -> Option<u8> {
        (self.phase >= 3).then_some(5)
    }
    pub fn is_triggered(&self) -> bool {
        self.phase >= 2
    }
    pub fn trigger(&mut self) {
        self.phase = 2;
    }
    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction;
    }
    pub fn tick(&mut self, e: &mut Enemy, target: Vec2, terrain: &impl CollisionField) {
        match e.definition {
            17 => match self.phase {
                0 => {
                    e.current_sprite = Some(84);
                    self.timer = 32;
                    self.remaining = 8;
                    self.phase = 1;
                }
                1 => {
                    self.timer -= 1;
                    if self.timer == 0 {
                        self.velocity.x = 0.625;
                        self.timer = 144;
                        self.phase = 2;
                    }
                }
                2 => {
                    e.position += self.velocity;
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer == 0 {
                        self.velocity = Vec2::default();
                    } else if self.timer == 240 {
                        self.timer = 129;
                        e.current_sprite = Some(85);
                        self.phase = 3;
                    }
                }
                3 => {
                    self.timer -= 1;
                    if self.timer == 0 {
                        self.timer = 129;
                        if self.remaining > 0 {
                            self.remaining -= 1;
                            e.spawns.push(EnemySpawn {
                                source_type: 1,
                                position: e.position + Vec2::new(-20., 0.),
                                direction: 0,
                                flips: (false, false),
                            });
                        }
                    }
                }
                _ => unreachable!(),
            },
            16 | 31 => match self.phase {
                0 => {
                    e.current_sprite = Some(0);
                    self.direction = if e.definition == 16 { 10 } else { 6 };
                    self.phase = 1;
                }
                1 => {
                    let tick = e.spawn_tick + u64::from(e.age - 1);
                    let screen = e.position - e.view_origin;
                    if tick.is_multiple_of(4)
                        && (0. ..256.).contains(&screen.x)
                        && (0. ..256.).contains(&screen.y)
                    {
                        let mut selected = target;
                        if let Some((positions, active)) = e.players {
                            if !active[usize::from(e.target_counter >> 7)] {
                                e.target_counter ^= 128;
                            }
                            selected = positions[usize::from(e.target_counter >> 7)];
                        }
                        let delta = |p: Vec2| {
                            Vec2::new(
                                p.x.floor() - e.position.x.floor(),
                                p.y.floor() - e.position.y.floor(),
                            )
                        };
                        let mut d = delta(selected);
                        if !(6..11).contains(&aim_direction(d)) {
                            e.target_counter ^= 128;
                            if let Some((positions, active)) = e.players {
                                if !active[usize::from(e.target_counter >> 7)] {
                                    e.target_counter ^= 128;
                                    return;
                                }
                                d = delta(positions[usize::from(e.target_counter >> 7)]);
                            }
                        }
                        if (6..11).contains(&aim_direction(d)) && d.x.abs() + d.y.abs() < 112. {
                            self.phase = 2;
                        }
                    }
                }
                2 => {
                    e.current_sprite = Some(25);
                    e.current_palette = Some(3);
                    self.timer = 24;
                    self.phase = 3;
                }
                3 => {
                    self.timer -= 1;
                    if self.timer == 0 {
                        e.current_sprite = Some(0);
                        self.phase = 4;
                    } else if self.timer.is_multiple_of(8) {
                        e.current_sprite = e.current_sprite.map(|s| s + 1);
                    }
                }
                4 => {
                    e.spawns.push(EnemySpawn {
                        source_type: 30,
                        position: e.position,
                        direction: self.direction,
                        flips: (self.direction & 8 != 0, false),
                    });
                    self.phase = 5;
                }
                5 => {}
                _ => unreachable!(),
            },
            30 => match self.phase {
                0 => {
                    e.current_sprite = Some(71);
                    self.timer = 32;
                    self.remaining = 48;
                    self.velocity = direction_velocity(self.direction, 1.75);
                    self.phase = 1;
                }
                1 => {
                    self.timer -= 1;
                    if self.timer.is_multiple_of(4) {
                        self.velocity.y = -self.velocity.y;
                    }
                    e.position += self.velocity;
                    if self.timer == 0 {
                        self.velocity = Vec2::new(0., 1.75);
                        e.current_sprite = Some(72);
                        self.phase = 2;
                    } else if self.timer.is_multiple_of(4) {
                        self.velocity.y = -self.velocity.y;
                    }
                }
                2 => {
                    self.remaining = self.remaining.wrapping_sub(1);
                    e.current_sprite = Some(if self.remaining & 8 != 0 || self.timer == 1 {
                        72
                    } else {
                        73
                    });
                    if terrain.terrain_at(super::infantry::lookahead(e.position, 8, true)) != 0 {
                        self.velocity = Vec2::default();
                    }
                    e.position += self.velocity;
                    if self.remaining == 0 || self.velocity == Vec2::default() {
                        self.velocity = Vec2::default();
                        self.timer = 1;
                        if e.actor_blocked(e.position) {
                            e.trigger_contact_death();
                        }
                    }
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
