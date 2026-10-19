//! Native boss encounter state machines.
//!
//! Part counts, hit points, explosive armor, spawn spacing, and the fifth boss's
//! protected core come from recovered actor data (analysis/native-recovery.txt).
//! Actor movement and attacks are recovered in dedicated modules and compared
//! against original state traces. Encounter integration coverage is tracked in
//! analysis/fidelity-audit.md.
use crate::{
    boss_hq::{HeadquartersBuilding, HeadquartersCenter, HeadquartersGuard, HeadquartersGun},
    boss_states::{Aircraft, BossActor, BossActorKind, BossAttack},
    boss_vehicle::ArmoredVehicle,
    math::Vec2,
    terrain::CollisionField,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    FortressGuns,
    BattleshipGuns,
    StatueBattery,
    AttackAircraft,
    HeavyTank,
    Headquarters,
}
impl BossKind {
    pub fn for_stage(stage: usize) -> Self {
        [
            Self::FortressGuns,
            Self::BattleshipGuns,
            Self::StatueBattery,
            Self::AttackAircraft,
            Self::HeavyTank,
            Self::Headquarters,
        ][stage.min(5)]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossPhase {
    Arriving,
    Attacking,
    CoreExposed,
    Defeated,
}
#[derive(Clone, Debug)]
pub struct BossPart {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub source_type: u8,
    pub position: [f32; 2],
    pub health: u16,
    pub max_health: u16,
    pub sprite: u16,
    pub alive: bool,
    pub active: bool,
    pub radius: f32,
    pub half_extents: Vec2,
    pub shielded: bool,
    pub flash: u8,
    pub grenade_only: bool,
    pub death_ticks: u8,
    death_elapsed: u8,
    target_counter: u8,
    target_initialized: bool,
    pub background_patch: Option<u8>,
    pub palette: Option<u8>,
    actor: Option<BossActor>,
    aircraft: Option<Aircraft>,
    vehicle: Option<ArmoredVehicle>,
    hq_center: Option<HeadquartersCenter>,
    hq_guard: Option<HeadquartersGuard>,
    hq_building: Option<HeadquartersBuilding>,
    hq_gun: Option<HeadquartersGun>,
    pub flip_x: bool,
    pub flip_y: bool,
    spawn_at: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct BossShot {
    pub configuration: u8,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub radius: f32,
    pub lifetime: u16,
    pub spawn_delay: u16,
    pub sprite: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct BossSpawn {
    pub source_type: u8,
    pub position: [f32; 2],
    pub direction: u8,
    pub variant: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct BossExplosion {
    pub position: [f32; 2],
}

pub struct Boss {
    pub controller: crate::boss_controller::BossController,
    pub completion_handles: Vec<crate::actor_pool::ActorHandle>,
    managed_actor_slots: bool,
    completed_by_helper: bool,
    manual_frame: bool,
    dispatch_part: Option<usize>,
    pub controller_handle: Option<crate::actor_pool::ActorHandle>,
    pub completion_handle: Option<crate::actor_pool::ActorHandle>,
    kind: BossKind,
    center: [f32; 2],
    pub parts: Vec<BossPart>,
    pub age: u32,
    pub phase: BossPhase,
    spawned: Vec<BossSpawn>,
    sound_commands: Vec<u8>,
    explosions: Vec<BossExplosion>,
    global_tick: u8,
    frame_context: Option<(Vec2, [Option<Vec2>; 2], u8)>,
    hq_transition: Option<u16>,
    hq_building_spawn_tick: Option<u16>,
}
impl Boss {
    pub fn new(kind: BossKind, center: [f32; 2]) -> Self {
        let mut boss = Self {
            controller: crate::boss_controller::BossController::new(kind, 0, center[1] - 64.),
            completion_handles: Vec::new(),
            managed_actor_slots: false,
            completed_by_helper: false,
            manual_frame: false,
            dispatch_part: None,
            controller_handle: None,
            completion_handle: None,
            kind,
            center,
            parts: Vec::new(),
            age: 0,
            phase: BossPhase::Arriving,
            spawned: Vec::new(),
            sound_commands: Vec::new(),
            explosions: Vec::new(),
            global_tick: 0,
            frame_context: None,
            hq_transition: None,
            hq_building_spawn_tick: None,
        };
        match kind {
            // Original controller schedules four armored tanks, rather than a
            // single fortress. The public name follows the current stage label.
            BossKind::FortressGuns => {
                for i in 0..4 {
                    let y = if i % 2 == 0 { 239. } else { 0. };
                    boss.part([-center[0], y - 64.], 10, 0, 18., true, 240 * (i + 1));
                    boss.parts.last_mut().unwrap().vehicle = Some(ArmoredVehicle::new(
                        10,
                        i as u8,
                        Vec2::new(0., center[1] - 64. + y),
                    ));
                    boss.parts
                        .last_mut()
                        .unwrap()
                        .vehicle
                        .as_mut()
                        .unwrap()
                        .set_entry_from_bottom(i % 2 == 0);
                }
            }
            BossKind::BattleshipGuns => {
                for (i, x) in [188.0, 268.0, 228.0, 308.0].iter().enumerate() {
                    boss.part(
                        [x - center[0], -36.0],
                        15,
                        0,
                        18.0,
                        true,
                        (4 - i) as u32 * 24,
                    );
                    boss.parts.last_mut().unwrap().actor =
                        Some(BossActor::new(BossActorKind::Tower, i as u8, 0));
                }
            }
            BossKind::StatueBattery => {
                for (i, (x, y)) in [
                    (112.0, 70.0),
                    (184.0, 54.0),
                    (184.0, 86.0),
                    (232.0, 54.0),
                    (232.0, 86.0),
                    (296.0, 70.0),
                ]
                .into_iter()
                .enumerate()
                {
                    boss.part(
                        [x - center[0], y - 64.0],
                        6,
                        0x63,
                        12.0,
                        true,
                        (6 - i) as u32 * 24,
                    );
                    boss.parts.last_mut().unwrap().actor =
                        Some(BossActor::new(BossActorKind::Statue, i as u8, 1));
                }
            }
            BossKind::AttackAircraft => {
                boss.part([-center[0], -64.], 32, 0, 25., true, 0);
                boss.parts[0].aircraft = Some(Aircraft::new(
                    Vec2::new(0., center[1] - 64.),
                    center[1] - 64.,
                ));
            }
            BossKind::HeavyTank => {
                for (i, x) in [128.0, 176.0, 384.0, 432.0].iter().enumerate() {
                    boss.part([x - center[0], 8.0], 3, 0, 14.0, true, (5 - i) as u32);
                    boss.parts.last_mut().unwrap().actor =
                        Some(BossActor::new(BossActorKind::Guard, i as u8, i as u8));
                }
                boss.part([280.0 - center[0], 0.0], 1, 0, 24.0, true, 1);
            }
            BossKind::Headquarters => {
                boss.part([256. - center[0], 46. - 64.], 63, 0, 22., true, 1);
                boss.parts[0].hq_center = Some(HeadquartersCenter::default());
                for (i, x) in [302., 210.].into_iter().enumerate() {
                    boss.part(
                        [x - center[0], 41. - 64.],
                        15,
                        0xca,
                        16.,
                        true,
                        i as u32 + 2,
                    );
                    boss.parts.last_mut().unwrap().hq_guard = Some(HeadquartersGuard::default());
                }
                boss.part(
                    [256. - center[0], 48. - 64.],
                    127,
                    0xd3,
                    31.,
                    true,
                    u32::MAX,
                );
                boss.parts[3].hq_building =
                    Some(HeadquartersBuilding::new(boss.parts[3].position.into()));
                boss.part([263. - center[0], 32. - 64.], 64, 0xfe, 16., true, u32::MAX);
                boss.parts[4].hq_gun = Some(HeadquartersGun::new(center[1] - 64.));
            }
        }
        for (i, part) in boss.parts.iter_mut().enumerate() {
            let (x, y) = match kind {
                BossKind::FortressGuns => (13., 18.),
                BossKind::BattleshipGuns => (10., 18.),
                BossKind::StatueBattery => (10., 10.),
                BossKind::AttackAircraft => (13., 25.),
                BossKind::HeavyTank if i == 4 => (10., 18.),
                BossKind::HeavyTank => (19., 17.),
                BossKind::Headquarters => {
                    [(13., 10.), (13., 18.), (13., 18.), (31., 15.), (10., 32.)][i]
                }
            };
            part.source_type = match kind {
                BossKind::FortressGuns => 10,
                BossKind::BattleshipGuns => 24,
                BossKind::StatueBattery => 146,
                BossKind::AttackAircraft => 192,
                BossKind::HeavyTank if i == 4 => 38,
                BossKind::HeavyTank => 49,
                BossKind::Headquarters => [74, 71, 71, 75, 79][i],
            };
            part.half_extents = Vec2::new(x, y);
            part.radius = x.max(y);
        }
        boss.refresh_shields();
        boss
    }
    fn part(
        &mut self,
        offset: [f32; 2],
        health: u16,
        sprite: u16,
        radius: f32,
        grenade_only: bool,
        spawn_at: u32,
    ) {
        self.parts.push(BossPart {
            actor_handle: None,
            source_type: 0,
            position: [self.center[0] + offset[0], self.center[1] + offset[1]],
            health,
            max_health: health,
            sprite,
            alive: true,
            active: false,
            radius,
            half_extents: Vec2::new(radius, radius),
            shielded: true,
            flash: 0,
            grenade_only,
            death_ticks: 0,
            death_elapsed: 0,
            target_counter: 0,
            target_initialized: false,
            background_patch: None,
            palette: None,
            actor: None,
            aircraft: None,
            vehicle: None,
            hq_center: None,
            hq_guard: None,
            hq_building: None,
            hq_gun: None,
            flip_x: false,
            flip_y: false,
            spawn_at,
        });
    }
    pub fn actor_pool_enabled(&self) -> bool {
        self.managed_actor_slots
    }
    pub fn part_occupies_slot(&self, index: usize) -> bool {
        let part = &self.parts[index];
        part.alive
            || part.death_ticks > 0
            || self.kind == BossKind::BattleshipGuns
            || (self.kind == BossKind::Headquarters
                && ((index == 0
                    && self
                        .hq_building_spawn_tick
                        .is_none_or(|spawn| self.hq_transition.is_none_or(|t| t < spawn + 96)))
                    || ((index == 1 || index == 2) && self.headquarters_guards_active())))
    }
    pub fn activate_part(&mut self, index: usize, handle: crate::actor_pool::ActorHandle) {
        self.parts[index].spawn_at = self.age;
        if self.kind == BossKind::Headquarters && index == 3 {
            self.hq_building_spawn_tick = self.hq_transition;
        }
        self.set_part_handle(index, handle);
    }
    pub fn enable_actor_pool(&mut self) {
        self.managed_actor_slots = true;
        if self.kind != BossKind::AttackAircraft {
            for part in &mut self.parts {
                part.spawn_at = u32::MAX;
            }
        }
    }
    pub fn tick_controller(
        &mut self,
        global_tick: u64,
    ) -> Option<crate::boss_controller::ControllerSpawn> {
        self.controller.tick(global_tick as u8)
    }
    pub fn resolve_controller_spawn(
        &mut self,
        request: crate::boss_controller::ControllerSpawn,
        handle: Option<crate::actor_pool::ActorHandle>,
    ) {
        if let Some(handle) = handle {
            if let Some(index) = request.part_index {
                self.parts[index].position = request.position.into();
                self.parts[index].spawn_at = self.age;
                self.set_part_handle(index, handle);
                if let Some(vehicle) = &mut self.parts[index].vehicle {
                    vehicle.position = request.position;
                }
            } else if request.source_type == 70 {
                self.completion_handles.push(handle);
            }
        }
        self.controller.resolve_spawn(handle.is_some());
    }
    pub fn tick_completion(
        &mut self,
        slot: u8,
        clock: u64,
        source_types: impl Iterator<Item = u8>,
    ) {
        if slot.wrapping_add(clock as u8) & 15 != 0 {
            return;
        }
        let required = match self.kind {
            BossKind::FortressGuns => 10,
            BossKind::BattleshipGuns => 24,
            BossKind::StatueBattery => 146,
            BossKind::AttackAircraft => 192,
            BossKind::HeavyTank => 38,
            BossKind::Headquarters => 75,
        };
        if !source_types.into_iter().any(|source| source == required) {
            self.completed_by_helper = true;
            self.refresh_shields();
        }
    }
    pub fn controller_source_type(&self) -> Option<u8> {
        match self.kind {
            BossKind::FortressGuns => Some(9),
            BossKind::BattleshipGuns => Some(32),
            BossKind::StatueBattery => Some(37),
            BossKind::AttackAircraft => None,
            BossKind::HeavyTank => Some(23),
            BossKind::Headquarters => Some(69),
        }
    }
    pub fn pending_part_indices(&self) -> Vec<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let due = p.spawn_at <= self.age.saturating_add(1);
                (p.alive && p.actor_handle.is_none() && due).then_some(i)
            })
            .collect()
    }
    pub fn set_part_handle(&mut self, index: usize, handle: crate::actor_pool::ActorHandle) {
        let part = &mut self.parts[index];
        part.actor_handle = Some(handle);
        part.target_counter = (handle.slot() & 1) << 7;
        if let Some(actor) = &mut part.actor {
            actor.set_slot(handle.slot());
        }
        if let Some(vehicle) = &mut part.vehicle {
            vehicle.set_slot(handle.slot());
        }
    }
    pub fn drain_sound_commands(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.sound_commands)
    }
    pub fn drain_explosions(&mut self) -> Vec<BossExplosion> {
        std::mem::take(&mut self.explosions)
    }
    pub fn drain_spawns(&mut self) -> Vec<BossSpawn> {
        std::mem::take(&mut self.spawned)
    }
    pub fn kind(&self) -> BossKind {
        self.kind
    }
    pub fn defeated(&self) -> bool {
        if self.completed_by_helper {
            return true;
        }
        if self.kind == BossKind::Headquarters {
            return !self.parts[3].alive && self.parts[3].death_ticks == 0;
        }
        self.parts
            .iter()
            .all(|part| !part.alive && part.death_ticks == 0)
    }
    fn refresh_shields(&mut self) {
        let core_unlocked = match self.kind {
            BossKind::HeavyTank => self.parts[..4]
                .iter()
                .all(|p| !p.alive && p.death_ticks == 0),
            BossKind::Headquarters => !self.parts[0].alive,
            _ => false,
        };
        for (i, part) in self.parts.iter_mut().enumerate() {
            part.shielded = !part.active
                || match self.kind {
                    BossKind::HeavyTank if i == 4 => !core_unlocked,
                    BossKind::Headquarters if i >= 3 => !core_unlocked,
                    BossKind::StatueBattery | BossKind::BattleshipGuns => {
                        part.actor.as_ref().is_some_and(|a| !a.vulnerable)
                    }
                    BossKind::HeavyTank => part.actor.as_ref().is_some_and(|a| !a.vulnerable),
                    _ => false,
                };
        }
        self.phase = if self.defeated() {
            BossPhase::Defeated
        } else if core_unlocked {
            BossPhase::CoreExposed
        } else if self.parts.iter().any(|p| p.active) {
            BossPhase::Attacking
        } else {
            BossPhase::Arriving
        };
    }
    pub fn configure(&mut self, difficulty: u8, global_tick: u64) {
        self.global_tick = global_tick as u8;
        self.controller = crate::boss_controller::BossController::new(
            self.kind,
            difficulty,
            self.center[1] - 64.,
        );
        if self.kind == BossKind::FortressGuns {
            let interval = [240, 208, 176, 144][usize::from(difficulty.min(3))];
            for (i, p) in self.parts.iter_mut().enumerate() {
                p.spawn_at = interval * (i as u32 + 1);
            }
        }
    }
    pub fn headquarters_guards_active(&self) -> bool {
        self.kind != BossKind::Headquarters || self.hq_transition.is_none_or(|tick| tick < 24)
    }
    pub fn headquarters_transition_tick(&self) -> Option<u16> {
        self.hq_transition
    }
    pub fn headquarters_final_death_tick(&self) -> Option<u16> {
        if self.kind != BossKind::Headquarters {
            return None;
        }
        let part = &self.parts[3];
        (!part.alive && part.death_elapsed > 0)
            .then_some(u16::from(part.death_elapsed.saturating_sub(1)))
    }
    pub fn headquarters_building_graphics(&self) -> Option<(Option<u8>, Option<u8>, u8, u16)> {
        let building = self.parts.iter().find_map(|p| p.hq_building.as_ref())?;
        building.graphics_phase.map(|phase| {
            (
                building.previous_previous_graphics_phase,
                building.previous_graphics_phase,
                phase,
                building.ticks_since_graphics,
            )
        })
    }
    pub fn headquarters_building_damage_phase(&self) -> u8 {
        self.parts
            .iter()
            .find_map(|p| p.hq_building.as_ref().map(|b| b.damage_phase))
            .unwrap_or(0)
    }
    pub fn set_deployed_vehicle_count(&mut self, count: u8) {
        for part in &mut self.parts {
            if let Some(actor) = &mut part.actor {
                actor.deployed_vehicles = count;
            }
        }
    }
    pub fn set_frame_context(
        &mut self,
        camera: Vec2,
        players: [Option<Vec2>; 2],
        global_tick: u64,
    ) {
        self.frame_context = Some((camera, players, global_tick as u8));
    }
    pub fn update(&mut self, target: [f32; 2]) -> Vec<BossShot> {
        self.update_inner(target, |_| (false, false))
    }
    pub fn update_with_terrain(
        &mut self,
        target: [f32; 2],
        terrain: &impl CollisionField,
        others: &[Vec2],
    ) -> Vec<BossShot> {
        self.update_inner(target, |vehicle| {
            let probe = vehicle.collision_position();
            (
                !terrain.traversable(vehicle.probe_position(), false),
                others.iter().any(|p| {
                    *p != vehicle.position
                        && (p.x - probe.x).abs() < 16.
                        && (p.y - probe.y).abs() < 16.
                }),
            )
        })
    }
    fn advance_frame(&mut self) {
        self.age = self.age.saturating_add(1);
        if self.kind == BossKind::Headquarters {
            if !self.parts[0].alive && self.hq_transition.is_none() {
                self.hq_transition = Some(0);
            } else if let Some(tick) = &mut self.hq_transition {
                *tick = tick.saturating_add(1);
            }
            if let Some(tick) = &mut self.hq_transition {
                if *tick == 24 {
                    for guard in &mut self.parts[1..3] {
                        guard.alive = false;
                        guard.death_ticks = 0;
                        guard.sprite = 0;
                    }
                }
                if *tick == 305 {
                    self.parts[3].spawn_at = self.age;
                }
            }
        }
    }
    pub fn begin_frame(&mut self) {
        assert!(!self.manual_frame, "boss frame already open");
        self.manual_frame = true;
        if !self.defeated() {
            self.advance_frame();
        }
    }
    pub fn tick_part(
        &mut self,
        index: usize,
        target: [f32; 2],
        terrain: &impl CollisionField,
        others: &[Vec2],
    ) -> Vec<BossShot> {
        assert!(self.manual_frame, "begin_frame must precede slot dispatch");
        self.dispatch_part = Some(index);
        let shots = self.update_with_terrain(target, terrain, others);
        self.dispatch_part = None;
        shots
    }
    pub fn finish_frame(&mut self) {
        self.manual_frame = false;
        self.dispatch_part = None;
        self.refresh_shields();
    }
    fn update_inner(
        &mut self,
        target: [f32; 2],
        mut blocked: impl FnMut(&ArmoredVehicle) -> (bool, bool),
    ) -> Vec<BossShot> {
        if self.defeated() {
            return Vec::new();
        }
        if !self.manual_frame {
            self.advance_frame();
        }
        let remaining = self
            .parts
            .iter()
            .filter(|p| p.alive || p.death_ticks > 0)
            .count();
        let (camera, players, clock) = self.frame_context.unwrap_or((
            Vec2::new(0., self.center[1] - 64.),
            [Some(target.into()), None],
            self.global_tick.wrapping_add(self.age as u8),
        ));
        let screen_x = players.map(|p| p.map_or(0, |p| (p.x - camera.x) as u8));
        let screen_y = players.map(|p| p.map_or(0, |p| (p.y - camera.y) as u8));
        let building_active = self.parts.get(3).is_none_or(|p| p.alive);
        let building_damaged = self.headquarters_building_damage_phase() > 0;
        let mut building_x = self.parts.get(3).map_or(0., |p| p.position[0]);
        let mut recovered_shots = Vec::new();
        for (i, part) in self.parts.iter_mut().enumerate() {
            if self.dispatch_part.is_some_and(|selected| selected != i) {
                continue;
            }
            part.flash = part.flash.saturating_sub(1);
            if !part.alive {
                if part.death_ticks > 0 {
                    let elapsed = part.death_elapsed;
                    part.death_elapsed = part.death_elapsed.saturating_add(1);
                    part.death_ticks -= 1;
                    let prep = matches!(
                        self.kind,
                        BossKind::FortressGuns | BossKind::BattleshipGuns | BossKind::StatueBattery
                    ) || (self.kind == BossKind::HeavyTank && i == 4);
                    let explosion_start = u8::from(prep);
                    if elapsed == 0
                        && ((prep && remaining == 1) || self.kind == BossKind::AttackAircraft)
                    {
                        self.sound_commands.push(0x59);
                    }
                    if self.kind == BossKind::Headquarters && i == 3 {
                        part.sprite = if part.death_ticks == 0 { 0xd4 } else { 0xd3 };
                        if elapsed == 0 {
                            self.sound_commands.push(0x27);
                        }
                    } else if self.kind == BossKind::AttackAircraft {
                        if elapsed == 0 {
                            for y in [part.position[1], part.position[1] - 32.] {
                                self.explosions.push(BossExplosion {
                                    position: [part.position[0], y],
                                });
                            }
                        }
                        if part.death_ticks == 0 {
                            part.sprite = 0;
                        }
                    } else if elapsed >= explosion_start {
                        let animation_tick = elapsed - explosion_start;
                        part.sprite = if animation_tick < 24 {
                            0x19 + u16::from(animation_tick / 8)
                        } else {
                            0
                        };
                        part.palette = Some(3);
                        if animation_tick == 0 {
                            self.sound_commands.push(0x17);
                        }
                    }
                    if self.kind == BossKind::BattleshipGuns && elapsed >= 1 {
                        part.background_patch = Some(6);
                    }
                    if part.death_ticks == 0 {
                        match self.kind {
                            BossKind::StatueBattery => part.background_patch = Some(14),
                            BossKind::HeavyTank => {
                                part.background_patch = Some(if i == 4 { 0 } else { 27 })
                            }
                            BossKind::Headquarters if i == 1 || i == 2 => part.sprite = 0xd2,
                            _ => {}
                        }
                    }
                }
                continue;
            }
            part.active = self.age >= part.spawn_at
                && (!self.managed_actor_slots || part.actor_handle.is_some());
            if !part.active {
                continue;
            }
            // F58E seeds selection by actor slot; F574 advances only in the
            // corresponding aiming state, not once per overall game frame.
            let advance_target = part.vehicle.as_ref().is_some_and(|v| v.state == 2)
                || part.aircraft.as_ref().is_some_and(|v| v.state == 1)
                || part
                    .actor
                    .as_ref()
                    .is_some_and(|v| v.kind == BossActorKind::Statue && v.state == 2)
                || part.hq_gun.as_ref().is_some_and(|v| v.state == 1);
            if advance_target {
                part.target_counter = part.target_counter.wrapping_sub(1);
            }
            if (!part.target_initialized || advance_target)
                && players[usize::from(part.target_counter >> 7)].is_none()
            {
                part.target_counter ^= 128;
            }
            part.target_initialized = true;
            let target: [f32; 2] =
                players[usize::from(part.target_counter >> 7)].map_or(target, Into::into);
            if let Some(center) = &mut part.hq_center {
                if let Some(spawn) =
                    center.tick(part.position.into(), part.health, clock, screen_y, camera.y)
                {
                    self.spawned.push(spawn);
                }
                part.sprite = center.sprite;
                part.palette = Some(center.palette);
                continue;
            }
            if let Some(guard) = &mut part.hq_guard {
                if let Some(spawn) = guard.tick(part.position.into(), clock, screen_x) {
                    self.spawned.push(spawn);
                }
                part.sprite = guard.sprite;
                part.flip_x = guard.flips.0;
                part.flip_y = guard.flips.1;
                continue;
            }
            if let Some(building) = &mut part.hq_building {
                if let Some(spawn) = building.tick(part.health, clock, screen_x, camera.x) {
                    if spawn.source_type != 79 {
                        self.spawned.push(spawn);
                    }
                }
                building_x = building.position.x;
                part.position = building.position.into();
                part.sprite = building.sprite;
                continue;
            }
            if let Some(gun) = &mut part.hq_gun {
                if let Some(shot) =
                    gun.tick_with_building(building_x, target.into(), building_active)
                {
                    recovered_shots.push(shot);
                }
                if gun.state == 5 {
                    part.alive = false;
                    part.health = 0;
                    part.death_ticks = 25;
                    part.death_elapsed = 0;
                }
                part.position = gun.position.into();
                part.sprite = gun.sprite;
                part.palette = Some(if building_damaged { 1 } else { 0 });
                part.flip_x = gun.flips.0;
                part.flip_y = gun.flips.1;
                continue;
            }
            if let Some(vehicle) = &mut part.vehicle {
                if vehicle.state == 0 && !self.managed_actor_slots {
                    let clock = self.global_tick.wrapping_add(self.age as u8);
                    vehicle.position.x = f32::from(clock.clamp(80, 176)) * 2.;
                }
                let (terrain_blocked, actor_blocked) = blocked(vehicle);
                if vehicle.tick(target.into(), terrain_blocked, actor_blocked) {
                    recovered_shots.push(BossShot {
                        position: vehicle.position.into(),
                        velocity: crate::enemy::direction_velocity(vehicle.direction, 1.75).into(),
                        radius: 3.,
                        configuration: 2,
                        lifetime: 64,
                        spawn_delay: 13,
                        sprite: 16,
                    });
                }
                part.position = vehicle.position.into();
                part.sprite = vehicle.sprite;
                part.flip_x = vehicle.flips.0;
                part.flip_y = vehicle.flips.1;
                continue;
            }
            if let Some(aircraft) = &mut part.aircraft {
                for attack in aircraft.tick(target.into()) {
                    match attack {
                        BossAttack::Bullet { velocity, .. } => recovered_shots.push(BossShot {
                            position: aircraft.position.into(),
                            velocity: velocity.into(),
                            radius: 3.,
                            configuration: 9,
                            lifetime: 48,
                            spawn_delay: 9,
                            sprite: 16,
                        }),
                        BossAttack::Spawn(source_type) => self.spawned.push(BossSpawn {
                            source_type,
                            position: [
                                aircraft.position.x
                                    + if aircraft.pod_direction == 0 {
                                        16.
                                    } else {
                                        -16.
                                    },
                                aircraft.position.y,
                            ],
                            direction: aircraft.pod_direction,
                            variant: aircraft.pod_index - 1,
                        }),
                        BossAttack::Shell => unreachable!(),
                    }
                }
                part.position = aircraft.position.into();
                part.sprite = aircraft.sprite;
                continue;
            }
            if let Some(actor) = &mut part.actor {
                let old_state = actor.state;
                for attack in actor.tick(
                    Vec2::new(target[0] - part.position[0], target[1] - part.position[1]),
                    remaining,
                ) {
                    match attack {
                        BossAttack::Bullet { velocity, .. } => recovered_shots.push(BossShot {
                            position: part.position,
                            velocity: velocity.into(),
                            radius: 3.,
                            configuration: 6,
                            lifetime: 136,
                            spawn_delay: 7,
                            sprite: 16,
                        }),
                        BossAttack::Shell => self.spawned.push(BossSpawn {
                            source_type: 57,
                            position: [part.position[0], part.position[1] + 16.],
                            direction: 8,
                            variant: 1,
                        }),
                        BossAttack::Spawn(source_type) => self.spawned.push(BossSpawn {
                            source_type,
                            position: [part.position[0], part.position[1] + 16.],
                            direction: 8,
                            variant: 0,
                        }),
                    }
                }
                if actor.kind == BossActorKind::Guard && (old_state == 3 || actor.state == 3) {
                    part.health = 3;
                }
                part.sprite = actor.sprite;
                part.background_patch = actor.patch;
                continue;
            }
            let age = self.age.saturating_sub(part.spawn_at);
            if self.kind == BossKind::HeavyTank && i == 4 {
                part.sprite = if remaining == 1 || age < 4 {
                    0
                } else {
                    0x93 + ((age / 4) % 8) as u16
                };
            }
        }
        if self.kind == BossKind::Headquarters
            && self.parts[3].active
            && self.parts[4].spawn_at == u32::MAX
        {
            self.parts[4].spawn_at = self.age + 1;
        }
        self.refresh_shields();
        recovered_shots
    }
    /// Returns true only when damage actually changes an exposed part's health.
    /// Explosions can hit several parts; a bullet stops at the first valid hit.
    pub fn damage(
        &mut self,
        position: [f32; 2],
        radius: f32,
        amount: u16,
        explosive: bool,
    ) -> bool {
        self.damage_area(&[position], radius, amount, explosive)
    }

    /// Each part takes damage once per attack, even when blast lobes overlap.
    pub fn damage_area(
        &mut self,
        positions: &[[f32; 2]],
        radius: f32,
        amount: u16,
        explosive: bool,
    ) -> bool {
        if amount == 0 || !radius.is_finite() || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return false;
        }
        let mut hit = false;
        for (i, part) in self.parts.iter_mut().enumerate() {
            if !part.alive || !part.active || part.shielded || (part.grenade_only && !explosive) {
                continue;
            }
            let reach = part.half_extents + Vec2::new(radius.max(0.), radius.max(0.));
            if !positions.iter().any(|position| {
                (part.position[0] - position[0]).abs() <= reach.x
                    && (part.position[1] - position[1]).abs() <= reach.y
            }) {
                continue;
            }
            part.health = part.health.saturating_sub(amount);
            part.alive = part.health > 0;
            if !part.alive {
                part.death_ticks = match self.kind {
                    BossKind::FortressGuns | BossKind::StatueBattery => 26,
                    BossKind::BattleshipGuns => 27,
                    BossKind::HeavyTank if i == 4 => 26,
                    BossKind::Headquarters if i == 3 => 208,
                    _ => 25,
                };
                part.death_elapsed = 0;
            }
            part.flash = 6;
            hit = true;
            if !explosive {
                break;
            }
        }
        self.refresh_shields();
        hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boss_death_frames_and_completion_match_original_handlers() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/boss_deaths.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let source = capture["source_type"].as_u64().unwrap();
            let (kind, index, initial_sprite) = match source {
                10 => (BossKind::FortressGuns, 0, 0x33),
                24 => (BossKind::BattleshipGuns, 0, 0),
                146 => (BossKind::StatueBattery, 0, 0x66),
                38 => (BossKind::HeavyTank, 4, 0),
                49 => (BossKind::HeavyTank, 0, 0xff),
                64 => (BossKind::AttackAircraft, 0, 0x7e),
                71 => (BossKind::Headquarters, 1, 0xca),
                74 => (BossKind::Headquarters, 0, 0xd1),
                _ => unreachable!(),
            };
            let mut boss = Boss::new(kind, [256., 64.]);
            for (i, p) in boss.parts.iter_mut().enumerate() {
                p.position = if i == index {
                    [80., 80.]
                } else {
                    [1000., 1000.]
                };
            }
            let part = &mut boss.parts[index];
            part.active = true;
            part.shielded = false;
            part.sprite = initial_sprite;
            assert!(boss.damage([80., 80.], 0., 999, true));
            for sample in capture["samples"].as_array().unwrap() {
                boss.update([80., 160.]);
                assert_eq!(
                    u64::from(boss.parts[index].sprite),
                    sample["sprite"].as_u64().unwrap(),
                    "type{source} tick{} sprite",
                    sample["tick"]
                );
                if source != 71 && source != 74 {
                    assert_eq!(
                        boss.parts[index].death_ticks == 0,
                        sample["boss_remaining"].as_u64().unwrap() < 6,
                        "type{source} tick{} completion",
                        sample["tick"]
                    );
                }
            }
        }
    }
    #[test]
    fn headquarters_preserves_guards_and_delayed_building_transition() {
        let mut boss = Boss::new(BossKind::Headquarters, [256., 64.]);
        for _ in 0..8 {
            boss.update([256., 180.]);
        }
        assert_eq!(boss.parts.len(), 5);
        assert!(boss.parts[..3].iter().all(|p| p.active));
        assert!(boss.parts[3..].iter().all(|p| !p.active));
        assert!(boss.damage([256., 46.], 0., 63, true));
        for _ in 0..24 {
            boss.update([256., 180.]);
        }
        assert!(boss.headquarters_guards_active());
        boss.update([256., 180.]);
        assert!(!boss.headquarters_guards_active());
        for _ in 24..304 {
            boss.update([256., 180.]);
        }
        assert!(!boss.parts[3].active);
        boss.update([256., 180.]);
        assert!(boss.parts[3].active);
        assert!(!boss.parts[4].active);
        boss.update([256., 180.]);
        assert!(boss.parts[4].active);
        assert_eq!(boss.parts[3].sprite, 0xd3);
    }
    #[test]
    fn overlapping_blast_lobes_damage_each_part_once() {
        let mut boss = Boss::new(BossKind::BattleshipGuns, [256.0, 80.0]);
        for _ in 0..200 {
            boss.update([256.0, 200.0]);
        }
        let position = boss.parts[0].position;
        let health = boss.parts[0].health;
        assert!(boss.damage_area(&[position, position, position], 0.0, 5, true));
        assert_eq!(boss.parts[0].health, health - 5);
    }
    #[test]
    fn armor_and_core_shields_require_destroying_outer_guns() {
        let mut boss = Boss::new(BossKind::HeavyTank, [256.0, 80.0]);
        for _ in 0..220 {
            boss.update([256.0, 200.0]);
        }
        let core = boss.parts[4].position;
        assert!(!boss.damage(core, 0.0, 999, true));
        for i in 0..4 {
            while boss.parts[i].shielded {
                boss.update([256.0, 200.0]);
            }
            let pos = boss.parts[i].position;
            assert!(!boss.damage(pos, 0.0, 999, false));
            assert!(boss.damage(pos, 0.0, 999, true));
        }
        for _ in 0..26 {
            boss.update([256., 200.]);
        }
        assert_eq!(boss.phase, BossPhase::CoreExposed);
        assert!(boss.damage(core, 0.0, 1, true));
        assert!(!boss.defeated());
        for _ in 0..26 {
            boss.update([256., 200.]);
        }
        assert!(boss.defeated());
    }
    #[test]
    fn every_encounter_can_finish_and_defeat_stops_projectiles() {
        for stage in 0..6 {
            let mut boss = Boss::new(BossKind::for_stage(stage), [256.0, 80.0]);
            let mut shots = 0;
            for tick in 0..2400 {
                shots += boss.update([256.0, 200.0]).len();
                shots += boss.drain_spawns().len();
                if tick > 1000 {
                    boss.damage([256.0, 80.0], 512.0, 10, true);
                }
                if boss.defeated() {
                    break;
                }
            }
            assert!(shots > 0, "stage{stage} never fired");
            assert!(boss.defeated(), "stage{stage} cannot be defeated");
            assert_eq!(boss.phase, BossPhase::Defeated);
            for _ in 0..200 {
                assert!(boss.update([256.0, 200.0]).is_empty());
                assert!(boss.drain_spawns().is_empty());
            }
        }
    }
    #[test]
    fn destroyed_parts_do_not_fire_while_survivors_continue() {
        let mut boss = Boss::new(BossKind::BattleshipGuns, [256.0, 80.0]);
        for _ in 0..100 {
            boss.update([256.0, 200.0]);
        }
        let position = boss.parts[0].position;
        assert!(boss.damage(position, 0.0, 15, true));
        let shots: Vec<_> = (0..300)
            .flat_map(|_| {
                boss.update([256.0, 200.0]);
                boss.drain_spawns()
            })
            .collect();
        assert!(!shots.is_empty());
        assert!(shots
            .iter()
            .all(|s| s.position != [position[0], position[1] + 16.]));
        assert!(shots
            .iter()
            .all(|s| s.position.iter().all(|v| v.is_finite())));
    }
    #[test]
    fn delayed_arrivals_cannot_be_skipped_by_an_early_blast() {
        let mut boss = Boss::new(BossKind::FortressGuns, [256.0, 80.0]);
        boss.update([256.0, 200.0]);
        boss.damage([256.0, 80.0], 500.0, 100, true);
        assert!(!boss.defeated());
        assert!(boss.parts.iter().any(|p| p.alive && !p.active));
    }
}
