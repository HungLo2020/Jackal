//! Proximity-revealed land mines, original actor35 (ACF8).
use crate::math::Vec2;
#[derive(Clone, Debug)]
pub struct LandMine {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub sprite: u16,
    pub palette: u8,
    pub alive: bool,
    death_age: Option<u8>,
    initialized: bool,
}
impl LandMine {
    pub fn new(position: Vec2) -> Self {
        Self {
            actor_handle: None,
            position,
            sprite: 0,
            palette: 0,
            alive: true,
            death_age: None,
            initialized: false,
        }
    }
    pub fn armed(&self) -> bool {
        self.alive && self.death_age.is_none()
    }
    pub fn detonate(&mut self) -> bool {
        if !self.armed() {
            return false;
        }
        self.death_age = Some(0);
        self.sprite = 25;
        self.palette = 3;
        true
    }
    pub fn tick(&mut self, players: [Option<Vec2>; 2]) {
        if !self.initialized {
            self.initialized = true;
            return;
        }
        if let Some(age) = self.death_age {
            let age = age + 1;
            self.death_age = Some(age);
            if age >= 24 {
                self.alive = false;
                self.sprite = 0;
            } else {
                self.sprite = 25 + u16::from(age / 8);
            }
            return;
        }
        let near = players.into_iter().flatten().any(|p| {
            (p.x.floor() - self.position.x.floor()).abs()
                + (p.y.floor() - self.position.y.floor()).abs()
                < 80.
        });
        self.sprite = if near { 115 } else { 0 };
        if near {
            self.palette = (self.palette + 1) & 3;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mines_reveal_by_distance_and_keep_original_death_frames() {
        let mut mine = LandMine::new(Vec2::new(100., 100.));
        mine.tick([Some(Vec2::new(100., 100.)), None]);
        assert_eq!(mine.sprite, 0);
        mine.tick([Some(Vec2::new(179., 100.)), None]);
        assert_eq!((mine.sprite, mine.palette), (115, 1));
        mine.tick([Some(Vec2::new(180., 100.)), None]);
        assert_eq!(mine.sprite, 0);
        assert!(mine.armed());
        assert!(mine.detonate());
        assert!(!mine.detonate());
        for tick in 1..24 {
            mine.tick([None, None]);
            assert_eq!(mine.sprite, 25 + tick / 8);
            assert_eq!(mine.palette, 3);
        }
        mine.tick([None, None]);
        assert!(!mine.alive);
    }
}
