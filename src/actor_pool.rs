//! The original sixteen general actor slots, without exposing storage addresses.
//!
//! Ordinary allocation searches15 down to0 (F25B). Priority allocation first
//! evicts the highest unprotected projectile/offscreen actor, then the highest
//! unprotected actor, finally slot0 when every actor is protected (F1EC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorHandle {
    slot: u8,
    generation: u64,
}
impl ActorHandle {
    pub fn slot(self) -> u8 {
        self.slot
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorMetadata {
    /// Original type, including its high-bit protection flag when present.
    pub source_type: u8,
    pub priority: bool,
    pub offscreen: bool,
}
impl ActorMetadata {
    pub fn new(source_type: u8, priority: bool, offscreen: bool) -> Self {
        Self {
            source_type,
            priority: priority || source_type & 128 != 0,
            offscreen,
        }
    }
    fn disposable(self) -> bool {
        self.offscreen || matches!(self.source_type, 52 | 54 | 57)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvictedActor {
    pub handle: ActorHandle,
    pub metadata: ActorMetadata,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub handle: ActorHandle,
    pub evicted: Option<EvictedActor>,
}
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    generation: u64,
    actor: Option<ActorMetadata>,
}
#[derive(Clone, Debug, Default)]
pub struct ActorPool {
    slots: [Slot; 16],
}
impl ActorPool {
    pub const CAPACITY: usize = 16;
    pub fn allocate(&mut self, metadata: ActorMetadata) -> Option<Allocation> {
        self.allocate_with_eviction(
            metadata,
            metadata.priority || metadata.source_type & 128 != 0,
        )
    }
    /// Runtime F266 child creation can fail even when the child is protected.
    pub fn allocate_child(&mut self, metadata: ActorMetadata) -> Option<Allocation> {
        self.allocate_with_eviction(metadata, false)
    }
    pub fn allocate_with_eviction(
        &mut self,
        metadata: ActorMetadata,
        force: bool,
    ) -> Option<Allocation> {
        let metadata =
            ActorMetadata::new(metadata.source_type, metadata.priority, metadata.offscreen);
        let free = (0..16).rev().find(|&i| self.slots[i].actor.is_none());
        let index = if let Some(index) = free {
            index
        } else if force {
            (0..16)
                .rev()
                .find(|&i| {
                    self.slots[i]
                        .actor
                        .is_some_and(|a| !a.priority && a.disposable())
                })
                .or_else(|| {
                    (0..16)
                        .rev()
                        .find(|&i| self.slots[i].actor.is_some_and(|a| !a.priority))
                })
                .unwrap_or(0)
        } else {
            return None;
        };
        let slot = &mut self.slots[index];
        let evicted = slot.actor.map(|metadata| EvictedActor {
            handle: ActorHandle {
                slot: index as u8,
                generation: slot.generation,
            },
            metadata,
        });
        slot.generation = slot
            .generation
            .checked_add(1)
            .expect("actor generation exhausted");
        slot.actor = Some(metadata);
        Some(Allocation {
            handle: ActorHandle {
                slot: index as u8,
                generation: slot.generation,
            },
            evicted,
        })
    }
    pub fn contains(&self, handle: ActorHandle) -> bool {
        let slot = &self.slots[usize::from(handle.slot)];
        slot.generation == handle.generation && slot.actor.is_some()
    }
    pub fn metadata(&self, handle: ActorHandle) -> Option<ActorMetadata> {
        self.contains(handle)
            .then(|| self.slots[usize::from(handle.slot)].actor)
            .flatten()
    }
    pub fn release(&mut self, handle: ActorHandle) -> bool {
        if !self.contains(handle) {
            return false;
        }
        self.slots[usize::from(handle.slot)].actor = None;
        true
    }
    pub fn update_offscreen(&mut self, handle: ActorHandle, offscreen: bool) -> bool {
        if !self.contains(handle) {
            return false;
        }
        self.slots[usize::from(handle.slot)]
            .actor
            .as_mut()
            .unwrap()
            .offscreen = offscreen;
        true
    }
    /// Update an existing slot when an actor changes type or protection in place.
    pub fn update_metadata(&mut self, handle: ActorHandle, metadata: ActorMetadata) -> bool {
        if !self.contains(handle) {
            return false;
        }
        self.slots[usize::from(handle.slot)].actor = Some(ActorMetadata::new(
            metadata.source_type,
            metadata.priority,
            metadata.offscreen,
        ));
        true
    }
    pub fn occupied(&self) -> impl Iterator<Item = (ActorHandle, ActorMetadata)> + '_ {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.actor.map(|metadata| {
                (
                    ActorHandle {
                        slot: i as u8,
                        generation: slot.generation,
                    },
                    metadata,
                )
            })
        })
    }
    pub fn len(&self) -> usize {
        self.occupied().count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Clear occupied actors while retaining generations so old handles stay stale.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.actor = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_children_do_not_force_eviction() {
        let mut pool = ActorPool::default();
        for _ in 0..16 {
            pool.allocate_child(ActorMetadata::new(1, false, false))
                .unwrap();
        }
        assert!(pool
            .allocate_child(ActorMetadata::new(146, true, false))
            .is_none());
        let event = pool.allocate(ActorMetadata::new(146, true, false)).unwrap();
        assert_eq!(event.handle.slot(), 15);
        assert!(event.evicted.is_some());
    }
    #[test]
    fn allocation_matches_four_hundred_original_pool_probes() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/actor_pool.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let mut pool = ActorPool::default();
            let handles: Vec<_> = (0..16)
                .map(|_| {
                    pool.allocate(ActorMetadata::new(1, false, false))
                        .unwrap()
                        .handle
                })
                .collect();
            for handle in handles {
                let i = usize::from(handle.slot());
                let source = case["types"][i].as_u64().unwrap() as u8;
                if source == 0 {
                    pool.release(handle);
                } else {
                    pool.update_metadata(
                        handle,
                        ActorMetadata::new(source, false, case["offscreen"][i].as_bool().unwrap()),
                    );
                }
            }
            let priority = case["priority"].as_bool().unwrap();
            let allocation = pool.allocate(ActorMetadata::new(1, priority, false));
            assert_eq!(
                allocation.map(|a| u64::from(a.handle.slot())),
                case["selected"].as_u64(),
                "case{} priority{priority}",
                case["case"]
            );
            if let Some(a) = allocation {
                if let Some(evicted) = a.evicted {
                    assert!(!pool.contains(evicted.handle));
                    assert!(!pool.release(evicted.handle));
                }
                assert!(pool.contains(a.handle));
                assert!(pool.len() <= 16);
            }
        }
    }
    #[test]
    fn stale_handles_cannot_remove_reused_slots() {
        let mut pool = ActorPool::default();
        let first = pool
            .allocate(ActorMetadata::new(1, false, false))
            .unwrap()
            .handle;
        assert_eq!(first.slot(), 15);
        assert!(pool.release(first));
        let second = pool
            .allocate(ActorMetadata::new(2, false, false))
            .unwrap()
            .handle;
        assert_eq!(second.slot(), 15);
        assert_ne!(first, second);
        assert!(!pool.release(first));
        assert!(pool.contains(second));
        pool.clear();
        assert!(!pool.contains(second));
        let third = pool
            .allocate(ActorMetadata::new(3, false, false))
            .unwrap()
            .handle;
        assert_ne!(second, third);
    }
    #[test]
    fn full_normal_allocation_does_not_mutate_or_evict() {
        let mut pool = ActorPool::default();
        for _ in 0..16 {
            assert!(pool.allocate(ActorMetadata::new(1, false, false)).is_some());
        }
        let before: Vec<_> = pool.occupied().collect();
        assert!(pool.allocate(ActorMetadata::new(2, false, false)).is_none());
        assert_eq!(before, pool.occupied().collect::<Vec<_>>());
    }
}
