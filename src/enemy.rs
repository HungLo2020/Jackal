//! Native actor state machines. Recovered sentry and launcher sequences are
//! checked against historical execution; remaining family ports are tracked in analysis.
mod angled_weapon;
pub mod death;
pub(crate) mod infantry;
mod retracting_gun;
mod rotating_turret;
mod scripted_vehicle;
mod side_gun;
pub(crate) mod support_vehicle;
use crate::{
    assets::{Assets, EntityDefinition},
    math::Vec2,
    terrain::CollisionField,
    weapon::Projectile,
};
use infantry::Infantry;
use rotating_turret::RotatingTurret;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    Infantry,
    Turret,
    Vehicle,
    Boat,
    Prison,
    Destructible,
    Helicopter,
    Environment,
}
impl Behavior {
    pub fn from_family(family: &str) -> Option<Self> {
        match family {
            "infantry" => Some(Self::Infantry),
            "turret" => Some(Self::Turret),
            "vehicle" => Some(Self::Vehicle),
            "boat" => Some(Self::Boat),
            "prison" => Some(Self::Prison),
            "destructible" => Some(Self::Destructible),
            "helicopter" => Some(Self::Helicopter),
            "environment" => Some(Self::Environment),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttackPhase {
    Initialize,
    Waiting,
    Opening,
    Firing,
    Closing,
}

#[derive(Clone, Debug)]
pub struct EnemySpawn {
    pub source_type: u8,
    pub position: Vec2,
    pub direction: u8,
    pub flips: (bool, bool),
}
#[derive(Clone, Debug)]
pub struct Enemy {
    pub position: Vec2,
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub definition: u8,
    pub health: u16,
    pub age: u32,
    pub flash: u8,
    pub alive: bool,
    pub behavior: Behavior,

    pub score: u32,
    view_origin: Vec2,
    players: Option<([Vec2; 2], [bool; 2])>,
    target_counter: u8,
    other_actors: Vec<(u8, Vec2)>,
    death_trigger: bool,
    spawns: Vec<EnemySpawn>,
    scripted: Option<scripted_vehicle::ScriptedVehicle>,
    vehicle: Option<crate::boss_vehicle::ArmoredVehicle>,
    rotating: Option<RotatingTurret>,
    stage: usize,
    difficulty: u8,
    boss_remaining: u8,
    mobile_launcher: Option<retracting_gun::MobileLauncher>,
    side_gun: Option<side_gun::SideGun>,
    support: Option<support_vehicle::SupportVehicle>,
    angled: Option<angled_weapon::AngledWeapon>,
    popup: Option<retracting_gun::PopupGun>,
    retracting: Option<retracting_gun::RetractingGun>,
    progression: u8,
    roaming: Option<Infantry>,
    spawn_tick: u64,
    attack_phase: AttackPhase,
    phase_ticks: u16,
    animation_step: u8,
    current_sprite: Option<u16>,
    current_palette: Option<u8>,
    sprite_flip_x: bool,
    sprite_flip_y: bool,
    spawn_slot: u8,
    rank: usize,
}
impl Enemy {
    pub fn new(def: &EntityDefinition, position: Vec2, behavior: Behavior) -> Self {
        Self {
            position,
            actor_handle: None,
            definition: def.source_type,
            health: def.hit_points.max(1),
            age: 0,
            flash: 0,
            alive: true,
            behavior,
            score: def.score,
            view_origin: Vec2::default(),
            players: None,
            target_counter: 0,
            other_actors: Vec::new(),
            death_trigger: false,
            spawns: Vec::new(),
            scripted: matches!(def.source_type, 16 | 17 | 30 | 31)
                .then(scripted_vehicle::ScriptedVehicle::default),
            vehicle: if matches!(def.source_type, 7 | 11 | 14 | 15) {
                Some(crate::boss_vehicle::ArmoredVehicle::new(
                    def.source_type,
                    0,
                    position,
                ))
            } else {
                None
            },
            rotating: if matches!(def.source_type, 5 | 6) {
                Some(RotatingTurret::default())
            } else {
                None
            },
            stage: 0,
            difficulty: 0,
            boss_remaining: 0,
            mobile_launcher: (def.source_type == 41).then(retracting_gun::MobileLauncher::default),
            side_gun: matches!(def.source_type, 33 | 34).then(side_gun::SideGun::default),
            support: matches!(def.source_type, 58 | 59 | 67)
                .then(|| support_vehicle::SupportVehicle::new(def.source_type, position)),
            angled: matches!(def.source_type, 42 | 43 | 47 | 48)
                .then(angled_weapon::AngledWeapon::default),
            popup: (def.source_type == 18).then(retracting_gun::PopupGun::default),
            retracting: (def.source_type == 12).then(retracting_gun::RetractingGun::default),
            progression: 0,
            roaming: if matches!(def.source_type, 1 | 4) {
                Some(Infantry::default())
            } else {
                None
            },
            spawn_tick: 0,
            attack_phase: AttackPhase::Initialize,
            phase_ticks: 0,
            animation_step: 0,
            current_sprite: None,
            current_palette: None,
            sprite_flip_x: false,
            sprite_flip_y: false,
            spawn_slot: 0,
            rank: 0,
        }
    }
    pub fn sprite(&self, assets: &Assets) -> Option<u16> {
        if let Some(sprite) = self.current_sprite {
            return Some(sprite);
        }
        let anim = &assets.entity(self.definition).animation;
        if anim.frames.is_empty() {
            None
        } else {
            Some(
                anim.frames
                    [(self.age / anim.frame_ticks.max(1) as u32) as usize % anim.frames.len()],
            )
        }
    }
    pub fn tick(
        &mut self,
        target: Vec2,
        level: &impl CollisionField,
        assets: &Assets,
    ) -> Vec<Projectile> {
        let target = self.select_target(target);
        let mut shots = self.tick_inner(target, level, assets);
        let configuration = match self.definition {
            5 => 12,
            6 => 13,
            7 | 8 | 26 => 1,
            11 => 3,
            12 => 7,
            18 => 4,
            42 | 43 => 8,
            67 => 10,
            _ => 0,
        };
        for shot in &mut shots {
            shot.configure_enemy_terrain(self.stage, configuration);
        }

        let screen = self.position - self.view_origin;
        if !(0. ..256.).contains(&screen.x) || !(0. ..256.).contains(&screen.y) {
            Vec::new()
        } else {
            shots
        }
    }
    pub fn set_boss_remaining(&mut self, count: u8) {
        self.boss_remaining = count;
    }
    pub fn terrain_patch(&self) -> Option<u8> {
        if let Some(gun) = &self.side_gun {
            return if self.alive { gun.patch } else { Some(7) };
        }
        if matches!(self.definition, 16 | 31) {
            self.scripted.as_ref().and_then(|s| s.terrain_patch())
        } else {
            None
        }
    }
    pub fn drain_sound_commands(&mut self) -> Vec<u8> {
        self.support
            .as_mut()
            .map_or_else(Vec::new, |s| s.drain_sound_commands())
    }
    pub fn drain_spawns(&mut self) -> Vec<EnemySpawn> {
        std::mem::take(&mut self.spawns)
    }
    pub fn set_spawn_palette(&mut self, palette: u8) {
        self.current_palette = Some(palette);
        if let Some(actor) = &mut self.support {
            actor.palette = palette;
        }
    }
    pub fn set_spawn_direction(&mut self, direction: u8, flips: (bool, bool)) {
        if let Some(actor) = &mut self.scripted {
            actor.set_direction(direction);
        }
        self.sprite_flip_x = flips.0;
        self.sprite_flip_y = flips.1;
    }
    /// Other occupied native actor slots, preserving the source type's high bit.
    /// Native scripted actors enter their death handler without awarding score.
    pub fn take_death_trigger(&mut self) -> bool {
        std::mem::take(&mut self.death_trigger)
    }
    fn trigger_contact_death(&mut self) {
        self.score = 0;
        self.death_trigger = true;
    }
    pub fn set_other_actors(&mut self, actors: Vec<(u8, Vec2)>) {
        self.other_actors = actors;
    }
    fn actor_blocked(&self, probe: Vec2) -> bool {
        self.other_actors.iter().any(|&(source, p)| {
            source > 0
                && source < 52
                && (-16. ..16.).contains(&(p.x.floor() - probe.x.floor()))
                && (-16. ..16.).contains(&(p.y.floor() - probe.y.floor()))
        })
    }
    pub fn set_players(&mut self, positions: [Vec2; 2], active: [bool; 2]) {
        self.players = Some((positions, active));
    }
    fn select_target(&mut self, fallback: Vec2) -> Vec2 {
        let Some((positions, active)) = self.players else {
            return fallback;
        };
        let advance = (matches!(self.definition, 2 | 3)
            && self.attack_phase == AttackPhase::Waiting)
            || (matches!(self.definition, 8 | 26) && self.age > 0)
            || self.vehicle.as_ref().is_some_and(|v| v.state == 2)
            || self.rotating.as_ref().is_some_and(|v| v.advances_target())
            || self.popup.as_ref().is_some_and(|v| v.advances_target())
            || self
                .mobile_launcher
                .as_ref()
                .is_some_and(|v| v.advances_target());
        if advance {
            self.target_counter = self.target_counter.wrapping_sub(1);
        }
        let refresh = self.age == 0
            || advance
            || self.roaming.as_ref().is_some_and(|v| v.refreshes_target())
            || self
                .retracting
                .as_ref()
                .is_some_and(|v| v.refreshes_target());
        if refresh && !active[usize::from(self.target_counter >> 7)] {
            self.target_counter ^= 128;
        }
        positions[usize::from(self.target_counter >> 7)]
    }
    pub fn set_view_origin(&mut self, origin: Vec2) {
        self.view_origin = origin;
    }
    fn tick_inner(
        &mut self,
        target: Vec2,
        level: &impl CollisionField,
        _assets: &Assets,
    ) -> Vec<Projectile> {
        self.age += 1;
        self.flash = self.flash.saturating_sub(1);
        if !self.alive {
            return Vec::new();
        }
        if let Some(mut actor) = self.scripted.take() {
            actor.tick(self, target, level);
            self.scripted = Some(actor);
            return Vec::new();
        }
        if let Some(mut actor) = self.angled.take() {
            let shots = actor.tick(self, target);
            self.angled = Some(actor);
            return shots;
        }
        if let Some(mut actor) = self.support.take() {
            actor.position = self.position;
            actor.set_rank(self.rank as u16);
            let shots = actor.tick(
                target,
                self.view_origin,
                self.spawn_tick + u64::from(self.age - 1),
            );
            self.position = actor.position;
            self.alive = actor.alive;
            self.current_sprite = Some(actor.sprite);
            self.current_palette = Some(actor.palette);
            self.sprite_flip_x = actor.flips.0;
            self.sprite_flip_y = actor.flips.1;
            self.support = Some(actor);
            return shots;
        }
        if let Some(mut actor) = self.side_gun.take() {
            let shots = actor.tick(self);
            self.side_gun = Some(actor);
            return shots;
        }
        if let Some(mut actor) = self.mobile_launcher.take() {
            let shots = actor.tick(self, target);
            self.mobile_launcher = Some(actor);
            return shots;
        }
        if let Some(mut gun) = self.popup.take() {
            let shots = gun.tick(self, target);
            self.popup = Some(gun);
            return shots;
        }
        if let Some(mut gun) = self.retracting.take() {
            let shots = gun.tick(self, target);
            self.retracting = Some(gun);
            return shots;
        }
        if matches!(self.definition, 8 | 26) {
            let tick = self.age - 1;
            let base = if self.definition == 8 { 58 } else { 86 };
            self.current_sprite =
                Some(base + u16::from(tick > 0 && (0u8.wrapping_sub(tick as u8) & 8 != 0)));
            if tick > 0 && tick <= 96 {
                let (heading, speed) = if self.definition == 8 {
                    (12, 0.5)
                } else {
                    (8, 0.75)
                };
                self.position += direction_velocity(heading, speed);
            }
            if tick > 0 && tick.is_multiple_of(48) {
                let heading = aim_direction(target - self.position);
                let heading = if self.definition == 8 {
                    ((heading + 1) & 31) / 4 * 4
                } else {
                    heading
                };
                return vec![Projectile::enemy_pattern(
                    self.position,
                    direction_velocity(heading, 1.125),
                    96 + self.rank as u16 * 8,
                    17,
                    16,
                )];
            }
            return Vec::new();
        }
        if let Some(mut vehicle) = self.vehicle.take() {
            vehicle.position = self.position;
            let blocked = level.terrain_at(vehicle.probe_position()) != 0;
            let actor_blocked = self.actor_blocked(vehicle.collision_position());
            let fire = vehicle.tick(target, blocked, actor_blocked);
            self.position = vehicle.position;
            self.current_sprite = Some(vehicle.sprite);
            self.sprite_flip_x = vehicle.flips.0;
            self.sprite_flip_y = vehicle.flips.1;
            if matches!(self.definition, 11 | 14) {
                self.current_palette = Some(if self.definition == 11 { 2 } else { 1 });
            }
            let shots = if fire {
                vec![if self.definition == 11 {
                    Projectile::enemy_pattern(
                        self.position,
                        direction_velocity(vehicle.direction, 1.5),
                        64 + self.rank as u16 * 8,
                        13,
                        74,
                    )
                } else if self.definition == 14 {
                    Projectile::enemy_grenade_pattern(self.position, vehicle.direction, 16)
                } else if self.definition == 7 {
                    Projectile::enemy_pattern(
                        self.position,
                        direction_velocity(vehicle.direction, 1.125),
                        96 + self.rank as u16 * 8,
                        17,
                        16,
                    )
                } else {
                    Projectile::enemy_bomb(
                        self.position,
                        aim_direction(target - self.position),
                        false,
                        self.spawn_tick + u64::from(self.age) - 1,
                    )
                }]
            } else {
                Vec::new()
            };
            self.vehicle = Some(vehicle);
            return shots;
        }
        if let Some(mut state) = self.rotating.take() {
            let shots = state.tick(self, target);
            self.rotating = Some(state);
            return shots;
        }
        if let Some(mut state) = self.roaming.take() {
            let shots = state.tick(self, target, level);
            self.roaming = Some(state);
            return shots;
        }
        if matches!(self.definition, 2 | 3) {
            return self.tick_sentry(target);
        }
        if matches!(self.definition, 44..=46) {
            return self.tick_launcher();
        }
        // Legacy damage fixtures can construct prison shells; live prison behavior
        // belongs to the typed rescue actors and never reaches this branch.
        assert_eq!(
            self.behavior,
            Behavior::Prison,
            "unsupported native enemy source {}",
            self.definition
        );
        Vec::new()
    }
    pub fn supports_source(source: u8) -> bool {
        matches!(source,1..=8|11|12|14..=18|26|30|31|33|34|41..=48|58|59|67)
    }
    /// Original spawn-slot staggering and stage/difficulty attack rank.
    pub fn configure_spawn(&mut self, slot: usize, stage: usize, difficulty: u8) {
        self.spawn_slot = (slot & 15) as u8;
        self.target_counter = if slot & 1 != 0 { 128 } else { 0 };
        if self.vehicle.is_some() {
            let mut v = crate::boss_vehicle::ArmoredVehicle::new(
                self.definition,
                (slot & 15) as u8,
                self.position,
            );
            v.set_stage(stage);
            self.vehicle = Some(v);
        }
        self.stage = stage;
        self.difficulty = difficulty;
        self.progression = (stage as u8).wrapping_add(difficulty);
        self.rank = ((stage + usize::from(difficulty)) / 2).min(5);
    }
    pub fn set_spawn_tick(&mut self, tick: u64) {
        self.spawn_tick = tick;
    }
    fn infantry_frame(&mut self, base: u16, direction: u8) {
        let octant = (((direction + 1) & 31) / 4) as usize;
        self.current_sprite = Some(base + [0, 0, 1, 1, 0, 0, 2, 2][octant]);
        self.sprite_flip_x = [false, false, false, false, true, true, false, false][octant];
        self.sprite_flip_y = false;
    }
    pub fn palette_override(&self) -> Option<u8> {
        self.current_palette
    }
    pub fn sprite_flips(&self) -> (bool, bool) {
        (self.sprite_flip_x, self.sprite_flip_y)
    }

    fn tick_sentry(&mut self, target: Vec2) -> Vec<Projectile> {
        let rank = if self.definition == 3 { 0 } else { self.rank };
        // Bank 6:84f8/851e/8570. Type 02 remains stationary and fires only
        // beyond a Manhattan distance of 64; it never chases the jeep.
        let direction = aim_direction(target - self.position);
        let octant = ((direction + 1) / 4) & 7;
        let attributes = [0x00u8, 0x0f, 0x0f, 0x00, 0x40, 0x4f, 0x00, 0x01][octant as usize];
        self.current_sprite = Some(18 + [0, 0, 1, 1, 0, 0, 2, 2][octant as usize]);
        self.sprite_flip_x = attributes & 0x40 != 0;
        match self.attack_phase {
            AttackPhase::Initialize => {
                self.current_sprite = Some(18);
                self.current_palette = Some(if self.definition == 3 { 3 } else { 1 });
                self.phase_ticks = u16::from(self.spawn_slot) * 8;
                if self.phase_ticks == 0 {
                    self.phase_ticks = 256;
                }
                self.attack_phase = AttackPhase::Waiting;
            }
            AttackPhase::Waiting => {
                self.phase_ticks -= 1;
                if self.phase_ticks == 0 {
                    self.phase_ticks = if self.definition == 3 {
                        128
                    } else {
                        [128, 112, 96, 80, 64, 48][rank]
                    };
                    let delta = target - self.position;
                    if delta.x.abs() + delta.y.abs() >= 64. {
                        self.phase_ticks = [8, 8, 16, 9, 24, 17][rank];
                        self.attack_phase = AttackPhase::Firing;
                    }
                }
            }
            AttackPhase::Firing => {
                // The signed countdown continues for seven recovery frames.
                self.phase_ticks = self.phase_ticks.wrapping_sub(1);
                if self.phase_ticks == u16::MAX - 6 {
                    self.attack_phase = AttackPhase::Waiting;
                    self.current_palette = Some(if self.definition == 3 { 3 } else { 1 });
                    self.phase_ticks = if self.definition == 3 {
                        128
                    } else {
                        [128, 112, 96, 80, 64, 48][rank]
                    };
                } else if self.phase_ticks.is_multiple_of(8) {
                    if self.definition == 3 {
                        return vec![Projectile::enemy_grenade(self.position, direction)];
                    }
                    let velocity = direction_velocity(direction, 1.0);
                    let shot =
                        Projectile::sentry_bullet(self.position, velocity, 64 + rank as u16 * 8);
                    return vec![shot];
                } else {
                    self.current_palette = self.current_palette.map(|p| (p + 1) & 3);
                }
            }
            _ => unreachable!("sentries have no opening/closing phase"),
        }
        Vec::new()
    }

    fn tick_launcher(&mut self) -> Vec<Projectile> {
        // 2C: A607..A680; 2D: A548..A5F2; 2E: B41F..B49E.
        let (base, step_ticks, opening_steps) = match self.definition {
            44 => (105, 8, 2),
            45 => (108, 4, 3),
            _ => (159, 16, 1),
        };
        if self.attack_phase == AttackPhase::Initialize {
            self.current_sprite = Some(base);
            self.phase_ticks = 128;
            self.attack_phase = AttackPhase::Waiting;
            return Vec::new();
        }
        self.phase_ticks -= 1;
        if self.attack_phase == AttackPhase::Waiting
            && self.definition == 45
            && !self.phase_ticks.is_multiple_of(4)
            && self.phase_ticks != 0
        {
            self.sprite_flip_x = !self.sprite_flip_x;
        }
        if self.phase_ticks != 0 {
            return Vec::new();
        }
        match self.attack_phase {
            AttackPhase::Waiting => {
                self.phase_ticks = step_ticks;
                self.animation_step = opening_steps;
                self.attack_phase = AttackPhase::Opening;
                if self.definition == 46 {
                    self.current_sprite = Some(base + 1);
                }
            }
            AttackPhase::Opening => {
                self.animation_step -= 1;
                if self.animation_step > 0 {
                    self.phase_ticks = step_ticks;
                    self.current_sprite = self.current_sprite.map(|frame| frame + 1);
                } else {
                    if self.definition == 44 {
                        self.current_sprite = Some(base);
                        self.phase_ticks = 128;
                        self.attack_phase = AttackPhase::Waiting;
                    } else {
                        self.phase_ticks = if self.definition == 45 { 4 } else { 8 };
                        self.animation_step = if self.definition == 45 { 3 } else { 1 };
                        self.attack_phase = AttackPhase::Closing;
                    }
                    return vec![Projectile::enemy_missile(self.position)];
                }
            }
            AttackPhase::Closing => {
                self.animation_step -= 1;
                if self.animation_step == 0 {
                    self.current_sprite = Some(base);
                    self.phase_ticks = 128;
                    self.attack_phase = AttackPhase::Waiting;
                } else {
                    self.current_sprite = self.current_sprite.map(|frame| frame - 1);
                    self.phase_ticks = step_ticks;
                }
            }
            _ => unreachable!("launcher state"),
        }
        Vec::new()
    }

    pub fn collision_half_size(&self, def: &EntityDefinition) -> Option<Vec2> {
        if matches!(self.definition, 16 | 31)
            && self.scripted.as_ref().is_some_and(|s| s.is_triggered())
        {
            return None;
        }
        if let Some(actor) = &self.mobile_launcher {
            return actor.vulnerable.then_some(Vec2::new(10., 10.));
        }
        if let Some(gun) = &self.popup {
            return gun.vulnerable.then_some(Vec2::new(10., 10.));
        }
        if let Some(gun) = &self.retracting {
            return gun.vulnerable.then_some(Vec2::new(13., 10.));
        }
        if self.definition == 46 && self.attack_phase == AttackPhase::Waiting {
            return None;
        }
        Some(Vec2::new(def.half_width, def.half_height))
    }
    pub fn damage(&mut self, amount: u16, explosive: bool, def: &EntityDefinition) -> bool {
        if self.collision_half_size(def).is_none()
            || !self.alive
            || self.behavior == Behavior::Environment
            || (self.definition == 46 && self.attack_phase == AttackPhase::Waiting)
            || (def.grenade_only && !explosive)
        {
            return false;
        }
        if matches!(self.definition, 16 | 31) {
            if let Some(actor) = &mut self.scripted {
                actor.trigger();
            }
            return true;
        }
        self.health = self.health.saturating_sub(amount);
        self.flash = 5;
        if self.health == 0 {
            self.alive = false;
        }
        true
    }
}

/// A defeated soldier is no longer a combat actor, but finishes its authored
/// one-shot death clip before disappearing. It cannot move, shoot or collide.
#[derive(Clone, Debug)]
pub struct InfantryDeath {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub age: u64,
}
impl InfantryDeath {
    pub fn frame(&self, assets: &Assets) -> Option<u16> {
        let clip = &assets.animations["infantry_death"];
        clip.frames
            .get((self.age / u64::from(clip.frame_ticks)) as usize)
            .copied()
    }
}

// The recovered direction table uses 32 headings, clockwise from east. Values
// are integer fixed-point coefficients, preserving the original diagonal speed.
pub(crate) fn direction_velocity(direction: u8, speed: f32) -> Vec2 {
    const MAGNITUDE: [i16; 16] = [
        128, 125, 118, 106, 90, 71, 48, 24, 0, 24, 48, 71, 90, 106, 118, 125,
    ];
    let d = direction & 31;
    let mut x = MAGNITUDE[(d & 15) as usize];
    let mut y = MAGNITUDE[((d + 8) & 15) as usize];
    if (8..24).contains(&d) {
        x = -x;
    }
    if d >= 16 {
        y = -y;
    }
    Vec2::new(
        (f32::from(x) * speed * 2.).floor() / 256.,
        (f32::from(y) * speed * 2.).floor() / 256.,
    )
}
pub(crate) fn aim_direction(delta: Vec2) -> u8 {
    // F5EE..F69E: quantized 8x8 slope table, including the original shared
    // fractional rounding term. This intentionally differs from atan2 rounding.
    const SLOPES: [u8; 32] = [
        0x40, 0, 0, 0, 0x84, 0x22, 0x11, 0x11, 0x86, 0x43, 0x22, 0x21, 0x86, 0x54, 0x33, 0x22,
        0x87, 0x65, 0x43, 0x33, 0x87, 0x65, 0x54, 0x43, 0x87, 0x66, 0x54, 0x44, 0x87, 0x76, 0x55,
        0x44,
    ];
    let mut x = (delta.x.abs() * 256.) as u32;
    let mut y = (delta.y.abs() * 256.) as u32;
    while x >= 7 * 256 || y >= 7 * 256 {
        x >>= 1;
        y >>= 1;
    }
    let round = u32::from(x & 255 >= 128);
    let index = ((y / 256 + round) * 8 + x / 256 + round) as usize;
    let entry = SLOPES[index / 2];
    let mut direction = i16::from(if index & 1 == 0 {
        entry >> 4
    } else {
        entry & 15
    });
    if delta.y < 0. {
        direction = -direction;
    }
    if delta.x < 0. {
        direction = 16 - direction;
    }
    direction.rem_euclid(32) as u8
}

#[cfg(test)]
mod actor_collision_tests {
    use super::*;
    #[test]
    fn original_actor_lookahead_uses_integer_coordinates_and_unmasked_source_type() {
        let assets =
            Assets::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
        let mut enemy = Enemy::new(assets.entity(7), Vec2::new(80., 80.), Behavior::Vehicle);
        let probe = Vec2::new(100.875, 120.125);
        for (source, delta, blocked) in [
            (7, Vec2::new(-16., 0.), true),
            (7, Vec2::new(15.9, 15.9), true),
            (7, Vec2::new(16., 0.), false),
            (7, Vec2::new(-16.1, 0.), false),
            (0, Vec2::ZERO, false),
            (51, Vec2::ZERO, true),
            (52, Vec2::ZERO, false),
            (135, Vec2::ZERO, false),
        ] {
            enemy.set_other_actors(vec![(source, Vec2::new(100., 120.) + delta)]);
            assert_eq!(
                enemy.actor_blocked(probe),
                blocked,
                "source {source} delta {delta:?}"
            );
        }
    }
}
