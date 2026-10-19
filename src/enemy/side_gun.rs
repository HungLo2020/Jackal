//! Background-mounted missile gun variants21/22, bank6 8EFF..8F92.
use super::*;
#[derive(Clone, Debug, Default)]
pub(super) struct SideGun {
    phase: u8,
    timer: u8,
    pub patch: Option<u8>,
    heading: u8,
}
impl SideGun {
    pub fn tick(&mut self, e: &mut Enemy) -> Vec<Projectile> {
        e.current_sprite = Some(0);
        if self.phase == 0 {
            self.timer = e.spawn_slot.wrapping_mul(4);
            self.heading = if e.position.x >= 256. { 12 } else { 4 };
            self.phase = 1;
            return Vec::new();
        }
        let visible_y = e.position.y - e.view_origin.y < 224.;
        match self.phase {
            1 => {
                if visible_y {
                    self.patch = Some(11);
                }
                if e.definition == 33 || e.difficulty >= 2 {
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer == 0 {
                        self.timer = 24;
                        self.phase = 2;
                    }
                }
            }
            2 => {
                self.timer -= 1;
                if visible_y {
                    self.patch = Some(if self.timer & 2 != 0 { 8 } else { 9 });
                }
                if self.timer == 0 {
                    self.timer = 24;
                    self.phase = 3;
                }
            }
            3 => {
                if visible_y {
                    self.patch = Some(10);
                }
                self.timer -= 1;
                if self.timer == 0 {
                    self.timer = 64;
                    self.phase = 1;
                    return vec![Projectile::enemy_missile_pattern(
                        e.position + Vec2::new(0., 16.),
                        self.heading,
                        0,
                        75,
                    )];
                }
            }
            _ => unreachable!(),
        }
        Vec::new()
    }
}
