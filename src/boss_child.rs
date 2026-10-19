//! Actors deployed by boss attacks: armored vehicles and aircraft paratroopers.
use crate::{
    boss::{BossShot, BossSpawn},
    boss_vehicle::ArmoredVehicle,
    enemy::direction_velocity,
    math::Vec2,
    terrain::CollisionField,
};
#[derive(Clone, Debug)]
enum ChildState {
    HeadquartersMissile {
        phase: u8,
        lifetime: u8,
        reveal: u8,
        frame: u16,
        velocity: Vec2,
    },
    HeadquartersBeam {
        initialized: bool,
        age: u8,
        launched: bool,
    },
    Vehicle(ArmoredVehicle),
    Paratrooper {
        phase: u8,
        timer: u8,
        velocity: Vec2,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum ChildEvent {
    Shot(BossShot),
    Infantry { position: Vec2, score_index: u8 },
}
#[derive(Clone, Debug)]
pub struct BossChild {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub source_type: u8,
    pub position: Vec2,
    pub sprite: u16,
    pub flips: (bool, bool),
    pub health: u16,
    pub alive: bool,
    pub death_ticks: u8,
    pub background_patch: Option<u8>,
    state: ChildState,
    camera: Vec2,
    target_counter: u8,
    target_initialized: bool,
}
impl BossChild {
    pub fn new(spawn: BossSpawn, slot: u8) -> Option<Self> {
        let state = match spawn.source_type {
            35 | 36 => ChildState::Vehicle(ArmoredVehicle::new(
                spawn.source_type,
                slot,
                spawn.position.into(),
            )),
            66 => ChildState::Paratrooper {
                phase: 0,
                timer: [8, 25, 42, 59][usize::from(spawn.variant & 3)],
                velocity: direction_velocity(spawn.direction, 1.),
            },
            72 => ChildState::HeadquartersMissile {
                phase: 0,
                lifetime: spawn.variant,
                reveal: 8,
                frame: if spawn.direction & 4 != 0 { 0xcf } else { 0xd0 },
                velocity: direction_velocity(spawn.direction, 2.75),
            },
            76 | 77 => ChildState::HeadquartersBeam {
                initialized: false,
                age: 0,
                launched: spawn.source_type == 77,
            },
            _ => return None,
        };
        Some(Self {
            actor_handle: None,
            source_type: spawn.source_type,
            position: spawn.position.into(),
            sprite: 0,
            flips: (spawn.source_type == 72 && spawn.direction & 12 == 12, false),
            health: if spawn.source_type == 35 { 6 } else { 4 },
            alive: true,
            death_ticks: 0,
            background_patch: None,
            state,
            camera: Vec2::ZERO,
            target_counter: (slot & 1) << 7,
            target_initialized: false,
        })
    }
    pub fn set_actor_handle(&mut self, handle: crate::actor_pool::ActorHandle) {
        self.actor_handle = Some(handle);
        self.target_counter = (handle.slot() & 1) << 7;
        if let ChildState::Vehicle(vehicle) = &mut self.state {
            vehicle.set_slot(handle.slot());
        }
    }
    pub fn select_target(&mut self, players: [Option<Vec2>; 2], fallback: Vec2) -> Vec2 {
        let advance = matches!(&self.state, ChildState::Vehicle(v) if v.state == 2);
        if advance {
            self.target_counter = self.target_counter.wrapping_sub(1);
        }
        if (!self.target_initialized || advance)
            && players[usize::from(self.target_counter >> 7)].is_none()
        {
            self.target_counter ^= 128;
        }
        self.target_initialized = true;
        players[usize::from(self.target_counter >> 7)].unwrap_or(fallback)
    }
    pub fn harmful_half_extents(&self) -> Option<Vec2> {
        if !self.alive || self.death_ticks > 0 {
            return None;
        }
        match self.source_type {
            72 => Some(Vec2::new(10., 10.)),
            76 | 77 => Some(Vec2::new(13., 18.)),
            _ => None,
        }
    }
    pub fn set_camera(&mut self, camera: Vec2) {
        self.camera = camera;
    }
    pub fn damage(&mut self, amount: u16) -> bool {
        if !self.alive || self.death_ticks > 0 || matches!(self.source_type, 66 | 72 | 76 | 77) {
            return false;
        }
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 {
            self.death_ticks = 24;
            self.sprite = 0x19;
        }
        true
    }
    pub fn tick(
        &mut self,
        target: Vec2,
        global_tick: u64,
        terrain: &impl CollisionField,
        other_actors: &[Vec2],
    ) -> Vec<ChildEvent> {
        let mut events = Vec::new();
        if !self.alive {
            return events;
        }
        if self.death_ticks > 0 {
            self.death_ticks -= 1;
            self.sprite = 0x19 + u16::from((24 - self.death_ticks).min(23) / 8);
            if self.death_ticks == 0 {
                self.sprite = 0;
                self.alive = false;
            }
            return events;
        }
        match &mut self.state {
            ChildState::HeadquartersMissile {
                phase,
                lifetime,
                reveal,
                frame,
                velocity,
            } => match *phase {
                0 => *phase = 1,
                1 => {
                    self.position += *velocity;
                    *lifetime = lifetime.wrapping_sub(1);
                    if *lifetime == 0 {
                        *phase = 2;
                    } else {
                        *reveal = reveal.wrapping_sub(1);
                        if *reveal == 0 {
                            self.sprite = *frame;
                        }
                    }
                }
                2 => {
                    self.background_patch = Some(26);
                    self.death_ticks = 24;
                    self.sprite = 0x19;
                    *phase = 3;
                }
                _ => {}
            },
            ChildState::HeadquartersBeam {
                initialized,
                age,
                launched,
            } => {
                if !*initialized {
                    *initialized = true;
                } else {
                    if self.source_type == 76 {
                        *age = age.wrapping_add(1);
                        self.sprite = 0xd5
                            + u16::from(age.wrapping_sub(1).min(15))
                            + if *age & 1 != 0 { 16 } else { 0 };
                        if *age == 16 {
                            *launched = true;
                        }
                    }
                    if *launched {
                        self.position.y += if self.source_type == 76 { 4. } else { 8. };
                    }
                    let x = self.position.x - self.camera.x;
                    if !(32.0..224.0).contains(&x) {
                        self.alive = false;
                    }
                }
            }
            ChildState::Vehicle(vehicle) => {
                let probe = vehicle.collision_position();
                let blocked = other_actors.iter().any(|p| {
                    *p != vehicle.position
                        && (p.x - probe.x).abs() < 16.
                        && (p.y - probe.y).abs() < 16.
                });
                if vehicle.tick(
                    target,
                    !terrain.traversable(vehicle.probe_position(), false),
                    blocked,
                ) {
                    let (lifetime, spawn_delay, speed, sprite) = if self.source_type == 35 {
                        (64, 13, 1.5, 74)
                    } else {
                        (96, 17, 0.75, 16)
                    };
                    events.push(ChildEvent::Shot(BossShot {
                        configuration: if self.source_type == 35 { 3 } else { 1 },
                        position: vehicle.position.into(),
                        velocity: direction_velocity(vehicle.direction, speed).into(),
                        radius: 3.,
                        lifetime,
                        spawn_delay,
                        sprite,
                    }));
                }
                self.position = vehicle.position;
                self.sprite = vehicle.sprite;
                self.flips = vehicle.flips;
            }
            ChildState::Paratrooper {
                phase,
                timer,
                velocity,
            } => match phase {
                0 => {
                    self.sprite = 0xab;
                    *phase = 1;
                }
                1 => {
                    self.position += *velocity;
                    *timer = timer.wrapping_sub(1);
                    if *timer == 0 {
                        self.sprite += 1;
                        *velocity = direction_velocity(6 + (global_tick as u8 & 3), 0.5);
                        *timer = 96;
                        *phase = 2;
                    }
                }
                2 => {
                    *timer = timer.wrapping_sub(1);
                    if *timer != 0 && *timer & 31 == 0 {
                        self.sprite += 1;
                    }
                    self.position += *velocity;
                    if *timer == 0 {
                        self.alive = false;
                        self.sprite = 0;
                        events.push(ChildEvent::Infantry {
                            position: self.position,
                            score_index: 15,
                        });
                    }
                }
                _ => unreachable!(),
            },
        }
        if matches!(self.source_type, 72 | 76 | 77)
            && (!(0.0..512.0).contains(&self.position.x)
                || !(-64.0..320.0).contains(&(self.position.y - self.camera.y)))
        {
            self.alive = false;
            self.sprite = 0;
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Ground;
    impl CollisionField for Ground {
        fn terrain_at(&self, _: Vec2) -> u8 {
            0
        }
    }
    #[test]
    fn vehicle_target_selection_matches_f574_counter_and_inactive_fallback() {
        let players = [Some(Vec2::new(32., 160.)), Some(Vec2::new(224., 160.))];
        for slot in [0, 1] {
            let mut child = BossChild::new(
                BossSpawn {
                    source_type: 35,
                    position: [128., 32.],
                    direction: 0,
                    variant: 0,
                },
                slot,
            )
            .unwrap();
            let initial = usize::from(slot & 1);
            assert_eq!(
                child.select_target(players, Vec2::ZERO),
                players[initial].unwrap()
            );
            // Entry movement does not call F574 and therefore does not age selection.
            for _ in 0..140 {
                assert_eq!(
                    child.select_target(players, Vec2::ZERO),
                    players[initial].unwrap()
                );
            }
            if let ChildState::Vehicle(vehicle) = &mut child.state {
                vehicle.state = 2;
            }
            let mut counter = (slot & 1) << 7;
            for _ in 0..260 {
                counter = counter.wrapping_sub(1);
                assert_eq!(
                    child.select_target(players, Vec2::ZERO),
                    players[usize::from(counter >> 7)].unwrap()
                );
            }
            assert_eq!(
                child.select_target([players[0], None], Vec2::ZERO),
                players[0].unwrap()
            );
        }
    }
    #[test]
    fn headquarters_children_match_original_flight() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/hq_children.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let typ = capture["source_type"].as_u64().unwrap() as u8;
            let mut child = BossChild::new(
                BossSpawn {
                    source_type: typ,
                    position: [80., 80.],
                    direction: capture["samples"][0]["direction"].as_u64().unwrap() as u8,
                    variant: if typ == 72 && capture["samples"][0]["direction"] == 8 {
                        68
                    } else if typ == 72 {
                        56
                    } else {
                        0
                    },
                },
                0,
            )
            .unwrap();
            for sample in capture["samples"].as_array().unwrap() {
                child.tick(Vec2::new(80., 160.), 0, &Ground, &[]);
                assert_eq!(
                    u64::from(child.sprite),
                    sample["sprite"].as_u64().unwrap(),
                    "type{typ} tick{} sprite",
                    sample["tick"]
                );
                assert_eq!(
                    child.position.x,
                    sample["x"].as_f64().unwrap() as f32,
                    "type{typ} tick{} x",
                    sample["tick"]
                );
                assert_eq!(
                    child.position.y,
                    sample["y"].as_f64().unwrap() as f32,
                    "type{typ} tick{} y",
                    sample["tick"]
                );
                if !child.alive {
                    break;
                }
            }
        }
    }
    #[test]
    fn paratrooper_deployment_matches_original_positions_and_frames() {
        let c: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/paratrooper_pod.json")).unwrap();
        let mut pod = BossChild::new(
            BossSpawn {
                source_type: 66,
                position: [80., 80.],
                direction: 0,
                variant: 0,
            },
            0,
        )
        .unwrap();
        for e in c["samples"].as_array().unwrap() {
            let events = pod.tick(Vec2::new(80., 160.), 0, &Ground, &[]);
            let tick = e["tick"].as_u64().unwrap();
            assert_eq!(
                u64::from(pod.sprite),
                e["sprite"].as_u64().unwrap(),
                "tick{tick} sprite"
            );
            assert_eq!(
                f64::from(pod.position.x),
                e["x"].as_f64().unwrap(),
                "tick{tick} x"
            );
            assert_eq!(
                f64::from(pod.position.y),
                e["y"].as_f64().unwrap(),
                "tick{tick} y"
            );
            assert_eq!(
                events.len(),
                e["shots"].as_array().unwrap().len(),
                "tick{tick} spawn"
            );
        }
        assert!(!pod.alive);
    }
}
