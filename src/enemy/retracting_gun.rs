//! Retracting gun (source0C), bank6 AC3A..ACF7.
use super::*;
#[derive(Clone, Debug, Default)]
pub(super) struct RetractingGun {
    phase: u8,
    timer: u8,
    recoil: u8,
    pub vulnerable: bool,
}
impl RetractingGun {
    pub fn refreshes_target(&self) -> bool {
        self.phase == 3
    }
    pub fn tick(&mut self, enemy: &mut Enemy, target: Vec2) -> Vec<Projectile> {
        match self.phase {
            0 => {
                enemy.current_sprite = Some(129);
                self.timer = 16;
                self.phase = 1;
            }
            1 | 4 => {
                self.timer -= 1;
                if self.timer == 0 {
                    self.vulnerable = true;
                    self.timer = 24;
                    self.phase += 1;
                }
            }
            2 => {
                if self.timer.is_multiple_of(8) {
                    enemy.current_sprite = enemy.current_sprite.map(|s| s + 1);
                }
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = 64 + enemy.difficulty * 8;
                    self.recoil = 0;
                    self.phase = 3;
                }
            }
            3 => {
                enemy.current_sprite = Some(if self.recoil > 0 {
                    self.recoil -= 1;
                    133
                } else {
                    132
                });
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = 8;
                    enemy.current_sprite = Some(132);
                    self.phase = 4;
                } else if self.timer.is_multiple_of(16) {
                    let direction = aim_direction(target - enemy.position);
                    if direction.wrapping_sub(2) < 14 {
                        self.recoil = 4;
                        return vec![Projectile::enemy_pattern(
                            enemy.position,
                            direction_velocity(direction, 1.75),
                            80 + enemy.rank as u16 * 8,
                            1,
                            74,
                        )];
                    }
                }
            }
            5 => {
                if self.timer.is_multiple_of(8) {
                    enemy.current_sprite = enemy.current_sprite.map(|s| s - 1);
                }
                self.timer -= 1;
                if self.timer == 0 {
                    self.vulnerable = false;
                    self.timer = 32;
                    self.phase = 1;
                }
            }
            _ => unreachable!(),
        }
        Vec::new()
    }
}

/// Source12 (hex), bank6 9592..96BD. The shared boss count controls recovery.
#[derive(Clone, Debug, Default)]
pub(super) struct PopupGun {
    phase: u8,
    timer: u8,
    fire_timer: u8,
    pub vulnerable: bool,
}
impl PopupGun {
    pub fn advances_target(&self) -> bool {
        self.phase == 2
    }
    pub fn tick(&mut self, e: &mut Enemy, target: Vec2) -> Vec<Projectile> {
        match self.phase {
            0 => {
                e.current_sprite = Some(if e.stage == 5 { 161 } else { 80 });
                e.current_palette = Some(2);
                self.timer = 8;
                self.fire_timer = e.spawn_slot.wrapping_mul(4);
                self.phase = 1;
            }
            1 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer >= 128 && self.timer != 192 && self.timer.is_multiple_of(16) {
                    self.vulnerable = true;
                    e.current_sprite = e.current_sprite.map(|s| s + 1);
                }
                if self.timer == 192 {
                    self.phase = 2;
                }
            }
            2 => {
                self.fire_timer = self.fire_timer.wrapping_sub(1);
                if self.fire_timer == 0 {
                    e.current_palette = Some(2);
                    self.fire_timer = 24;
                    let heading = aim_direction(target - e.position);
                    self.timer = 16;
                    self.phase = 3;
                    return (0..5)
                        .map(|i| {
                            Projectile::enemy_pattern(
                                e.position,
                                direction_velocity(
                                    heading.wrapping_sub(4).wrapping_add(i * 2) & 31,
                                    1.75,
                                ),
                                96 + e.rank as u16 * 8,
                                7,
                                16,
                            )
                        })
                        .collect();
                } else if self.fire_timer < 9 {
                    e.current_palette = e.current_palette.map(|p| (p + 1) & 3);
                }
            }
            3 => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer >= 128 && self.timer != 192 && self.timer.is_multiple_of(16) {
                    e.current_sprite = e.current_sprite.map(|s| s - 1);
                }
                if self.timer == 192 {
                    self.vulnerable = false;
                    self.timer = if e.boss_remaining < 2 {
                        self.fire_timer = 9;
                        1
                    } else {
                        80
                    };
                    self.phase = 1;
                }
            }
            _ => unreachable!(),
        }
        Vec::new()
    }
}

/// Source29, a retracting missile emplacement that relocates upward three times.
#[derive(Clone, Debug, Default)]
pub(super) struct MobileLauncher {
    phase: u8,
    timer: u8,
    base: u16,
    relocations: u8,
    pub vulnerable: bool,
}
impl MobileLauncher {
    pub fn advances_target(&self) -> bool {
        self.phase == 3
    }
    pub fn tick(&mut self, e: &mut Enemy, target: Vec2) -> Vec<Projectile> {
        match self.phase {
            0 => {
                e.current_sprite = Some(94);
                self.timer = 32;
                self.phase = 1;
            }
            1 => {
                e.current_sprite = Some(if self.timer & 1 == 0 { 0 } else { 94 });
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = 32;
                    self.base = 88;
                    e.current_sprite = Some(88);
                    self.vulnerable = true;
                    self.phase = 2;
                }
            }
            2 => {
                e.current_sprite = Some(self.base);
                self.timer -= 1;
                if self.timer.is_multiple_of(16) {
                    self.base += 1;
                }
                if self.timer & 1 != 0 {
                    e.current_sprite = e.current_sprite.map(|s| s + 3);
                }
                if self.timer == 0 {
                    self.timer = 32;
                    self.phase = 3;
                }
            }
            3 | 4 => {
                e.current_sprite = Some(self.base + if self.timer & 1 != 0 { 3 } else { 0 });
                self.timer -= 1;
                if self.timer == 0 {
                    if self.phase == 3 {
                        self.timer = 64;
                        self.phase = 4;
                        let heading = ((aim_direction(target - e.position) + 1) & 31) / 4 * 4;
                        return vec![Projectile::enemy_missile_pattern(
                            e.position, heading, 3, 75,
                        )];
                    }
                    self.timer = 48;
                    self.phase = 5;
                }
            }
            5 => {
                e.current_sprite = Some(self.base);
                self.timer -= 1;
                if self.timer > 0 {
                    if self.timer.is_multiple_of(16) {
                        self.base -= 1;
                    }
                    if self.timer & 1 != 0 {
                        e.current_sprite = e.current_sprite.map(|s| s + 3);
                    }
                }
                if self.timer == 0 {
                    self.vulnerable = false;
                    e.current_sprite = Some(94);
                    if self.relocations == 3 || e.position.y - e.view_origin.y < 0. {
                        self.timer = 96;
                        self.phase = 1;
                    } else {
                        self.relocations += 1;
                        self.timer = 128;
                        self.phase = 6;
                    }
                }
            }
            6 => {
                e.current_sprite = Some(if self.timer & 1 == 0 { 0 } else { 94 });
                e.position.y -= 0.5;
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = 96;
                    self.phase = 1;
                }
            }
            _ => unreachable!(),
        }
        Vec::new()
    }
}
