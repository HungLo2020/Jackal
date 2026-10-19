//! Concealed bonuses: shoot the hiding place, then collect the revealed icon.
use crate::math::Vec2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Bomb,
    Weapon,
    ExtraLife,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Hidden,
    Reveal,
    Visible,
}
#[derive(Clone, Debug)]
pub struct Pickup {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub kind: PickupKind,
    pub palette: u8,
    pub score: u32,
    phase: Phase,
}
impl Pickup {
    pub fn new(position: Vec2, kind: PickupKind, stage: usize, cooperative: bool) -> Self {
        Self {
            actor_handle: None,
            position,
            kind,
            palette: 0,
            score: if cooperative { 8000 } else { 5000 },
            phase: if stage == 2 && kind == PickupKind::Weapon {
                Phase::Reveal
            } else {
                Phase::Hidden
            },
        }
    }
    pub fn reveal(&mut self) -> bool {
        if self.phase != Phase::Hidden {
            return false;
        }
        self.phase = Phase::Reveal;
        true
    }
    pub fn visible(&self) -> bool {
        self.phase == Phase::Visible
    }
    pub fn tick(&mut self) {
        match self.phase {
            Phase::Hidden => {}
            Phase::Reveal => {
                self.palette = if self.kind == PickupKind::ExtraLife {
                    0
                } else {
                    1
                };
                self.phase = Phase::Visible;
            }
            Phase::Visible => {
                if self.kind == PickupKind::Weapon {
                    self.palette = (self.palette + 1) & 3;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bonus_is_hidden_until_shot_and_reveals_only_once() {
        let mut p = Pickup::new(Vec2::ZERO, PickupKind::Bomb, 0, false);
        p.tick();
        assert!(!p.visible());
        assert!(p.reveal());
        assert!(!p.reveal());
        p.tick();
        assert!(p.visible());
        assert_eq!(p.palette, 1);
        assert_eq!(p.score, 5000);
    }
    #[test]
    fn third_stage_weapon_bonus_begins_revealed_and_cycles_palette() {
        let mut p = Pickup::new(Vec2::ZERO, PickupKind::Weapon, 2, true);
        p.tick();
        assert!(p.visible());
        assert_eq!(p.palette, 1);
        p.tick();
        assert_eq!(p.palette, 2);
        assert_eq!(p.score, 8000);
    }
}
