//! Background scenery that can be shot, without inventing a firing enemy.
use crate::{assets::EntityDefinition, math::Vec2};
#[derive(Clone, Debug)]
pub struct BackgroundObject {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub source_type: u8,
    pub health: u16,
    pub half: Vec2,
    pub score: u32,
    pub death_age: Option<u8>,
    pub patch: u8,
    pub sound: Option<u8>,
}
impl BackgroundObject {
    pub fn new(position: Vec2, def: &EntityDefinition, stage: usize) -> Self {
        Self {
            actor_handle: None,
            position,
            source_type: def.source_type,
            health: def.hit_points,
            half: Vec2::new(def.half_width, def.half_height),
            score: def.score,
            death_age: None,
            patch: if def.source_type == 27 {
                0
            } else if stage == 2 {
                28
            } else {
                24
            },
            sound: if def.source_type == 27 {
                None
            } else {
                Some(0x17)
            },
        }
    }
    pub fn hit(&mut self, damage: u16) -> bool {
        if self.health == 0 {
            return false;
        }
        self.health = self.health.saturating_sub(damage);
        if self.health == 0 {
            self.death_age = Some(0);
        }
        true
    }
    pub fn tick(&mut self) -> bool {
        if let Some(age) = &mut self.death_age {
            if *age < 24 {
                *age += 1;
                return *age == 24 && self.source_type == 83;
            }
        }
        false
    }
    pub fn sprite(&self) -> Option<u16> {
        self.death_age
            .filter(|&age| age < 24)
            .map(|age| 25 + u16::from(age / 8))
    }
}
