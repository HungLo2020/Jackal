//! Fixed spread launchersA2A3..A36C and moving cannon trapsA381..A46F.
use super::*;
#[derive(Clone, Debug, Default)]
pub(super) struct AngledWeapon {
    phase: u8,
    timer: u8,
    heading: u8,
    spread: u8,
    speed_step: usize,
    attributes: u8,
}
impl AngledWeapon {
    pub fn tick(&mut self, e: &mut Enemy, target: Vec2) -> Vec<Projectile> {
        if matches!(e.definition, 42 | 43) {
            if self.phase == 0 {
                e.current_sprite = Some(103);
                e.sprite_flip_x = e.definition == 43;
                self.heading = if e.definition == 42 { 4 } else { 12 };
                self.timer = 4;
                self.phase = 1;
                return Vec::new();
            }
            self.timer = self.timer.wrapping_sub(1);
            e.current_sprite = Some(103 + u16::from(self.phase > 1 && self.timer >= 128));
            if self.phase == 1 {
                if self.timer == 0 {
                    self.timer = 5;
                    self.phase = 2;
                }
                return Vec::new();
            }
            if self.timer == 0 {
                let headings = if self.phase == 2 {
                    vec![self.heading]
                } else {
                    vec![self.heading + self.spread, self.heading - self.spread]
                };
                return headings
                    .into_iter()
                    .map(|h| {
                        Projectile::enemy_pattern(
                            e.position,
                            direction_velocity(h, 2.),
                            64 + e.rank as u16 * 8,
                            9,
                            74,
                        )
                    })
                    .collect();
            }
            if self.timer == 252 {
                self.timer = 5;
                self.spread += 1;
                self.phase += 1;
                if self.phase == 5 {
                    self.phase = 1;
                    self.timer = 48;
                    self.spread = 0;
                }
            }
        } else {
            match self.phase {
                0 => {
                    e.current_sprite = Some(124);
                    self.attributes = (e.spawn_tick as u8) & 192;
                    self.timer = 96;
                    self.phase = 1;
                }
                1 => {
                    let tick = e.spawn_tick + u64::from(e.age - 1);
                    let target = if let Some((positions, active)) = e.players {
                        let first_dy =
                            (positions[0].y.floor() as i32 - e.position.y.floor() as i32) as u8;
                        if active[0] && first_dy < 64 {
                            positions[0]
                        } else if active[1] {
                            positions[1]
                        } else {
                            return Vec::new();
                        }
                    } else {
                        target
                    };
                    let dy = (target.y.floor() as i32 - e.position.y.floor() as i32) as u8;
                    if tick.is_multiple_of(4) && dy < 64 {
                        let mut h = aim_direction(target - e.position);
                        let adjusted = h.wrapping_add(4) & 31;
                        if adjusted & 8 == 0 || adjusted & 7 == 0 {
                            if (e.definition == 47 && adjusted & 16 == 0)
                                || (e.definition == 48 && adjusted & 16 != 0)
                            {
                                h = 0u8.wrapping_sub(h.wrapping_add(16)) & 31;
                            }
                            self.heading = h;
                            self.phase = 2;
                        }
                    }
                }
                2 => {
                    self.timer -= 1;
                    let offset = if self.timer & 1 != 0 {
                        self.attributes = self.attributes.wrapping_add(64);
                        30
                    } else {
                        2
                    };
                    e.position += direction_velocity(
                        self.heading.wrapping_add(offset) & 31,
                        [0.5, 1., 2.5, 2., 1., 0.5][self.speed_step],
                    );
                    if self.timer == 48 {
                        e.current_sprite = Some(125);
                    }
                    if self.timer == 0 {
                        self.phase = 3;
                    } else if self.timer.is_multiple_of(16) {
                        self.speed_step += 1;
                    }
                }
                3 => {
                    if e.actor_blocked(e.position) {
                        e.trigger_contact_death();
                    }
                }
                _ => unreachable!(),
            }
            e.sprite_flip_x = self.attributes & 64 != 0;
            e.sprite_flip_y = self.attributes & 128 != 0;
        }
        Vec::new()
    }
}
