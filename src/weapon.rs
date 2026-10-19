//! Weapons use positions, velocities and explicit lifetimes, independent of rendering.
mod player_projectile;
use crate::{
    input::PlayerInput,
    math::{segment_hits_circle, Vec2},
    player::{Player, WeaponLevel},
};
use player_projectile::PlayerFlight;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileKind {
    Bullet,
    Grenade,
    Rocket,
    EnemyBullet,
    EnemyGrenade,
    EnemyMissile,
    BlastWave,
}
#[derive(Clone, Debug)]
enum EnemyFlight {
    Bomb {
        timer: u16,
        frame: u16,
        attributes: u8,
        explosion_ticks: Option<u8>,
    },
    Bullet {
        direction: u8,
        configuration: u8,
        terrain_collision: bool,
        marsh_collision: bool,
        timer: u16,
        travel_frame: u16,
        delay: u16,
        impact_ticks: Option<u8>,
        frame: Option<u16>,
    },
    Grenade {
        delay: u16,
        direction: u8,
        frame: Option<u16>,
        flips: (bool, bool),
    },
    Missile {
        direction: u8,
        speed: f32,
        maximum_turns: u8,
        base_frame: u16,
        turn_ticks: u8,
        turn_interval: u8,
        turns: u8,
        explosion_ticks: Option<u8>,
        frame: u16,
        flips: (bool, bool),
    },
}
#[derive(Clone, Debug)]
pub struct Projectile {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub player_slot: Option<u8>,
    enemy_state: Option<EnemyFlight>,
    player_state: Option<PlayerFlight>,
    pub position: Vec2,
    pub previous: Vec2,
    pub velocity: Vec2,
    pub kind: ProjectileKind,
    pub owner: Option<usize>,
    pub age: u16,
    pub remaining: u16,
    pub upgrade: WeaponLevel,
    enemy_palette: u8,
    players: Option<([Vec2; 2], [bool; 2])>,
    target_counter: u8,
}
impl Projectile {
    pub fn new(
        position: Vec2,
        velocity: Vec2,
        kind: ProjectileKind,
        owner: Option<usize>,
        lifetime: u16,
        upgrade: WeaponLevel,
    ) -> Self {
        Self {
            actor_handle: None,
            player_slot: None,
            enemy_state: None,
            player_state: None,
            position,
            previous: position,
            velocity,
            kind,
            owner,
            age: 0,
            remaining: lifetime,
            upgrade,
            enemy_palette: 2,
            players: None,
            target_counter: 0,
        }
    }
    pub fn enemy(position: Vec2, velocity: Vec2) -> Self {
        Self::new(
            position,
            velocity,
            ProjectileKind::EnemyBullet,
            None,
            240,
            WeaponLevel::Grenade,
        )
    }
    pub fn sentry_bullet(position: Vec2, velocity: Vec2, lifetime: u16) -> Self {
        Self::enemy_pattern(position, velocity, lifetime, 9, 16)
    }
    pub fn enemy_pattern(
        position: Vec2,
        velocity: Vec2,
        lifetime: u16,
        spawn_delay: u16,
        sprite: u16,
    ) -> Self {
        let mut shot = Self::enemy(position, velocity);
        shot.remaining = lifetime + 9;
        let direction = (0u8..32)
            .min_by(|a, b| {
                let error = |d| {
                    let v = crate::enemy::direction_velocity(d, 1.).normalized()
                        - velocity.normalized();
                    v.x * v.x + v.y * v.y
                };
                error(*a).total_cmp(&error(*b))
            })
            .unwrap();
        shot.enemy_state = Some(EnemyFlight::Bullet {
            direction,
            configuration: 0,
            terrain_collision: true,
            marsh_collision: false,
            timer: lifetime,
            travel_frame: sprite,
            delay: spawn_delay,
            impact_ticks: None,
            frame: None,
        });
        shot
    }
    /// Source36 checks walls on odd remaining flight ticks; late configurations
    /// skip collision, and stage6 configurations below10 also stop on marsh.
    pub fn configure_enemy_terrain(&mut self, stage: usize, configuration: u8) {
        if let Some(EnemyFlight::Bullet {
            configuration: saved_configuration,
            terrain_collision,
            marsh_collision,
            ..
        }) = &mut self.enemy_state
        {
            self.enemy_palette = [2, 2, 3, 2, 2, 2, 3, 2, 2, 2, 2, 3, 2, 3, 2, 3]
                [usize::from(configuration.min(15))];
            *saved_configuration = configuration;
            *terrain_collision = configuration < 14;
            *marsh_collision = stage == 5 && configuration < 10;
        }
    }
    /// BD C1..D6: the stage-five final approach disables wall collision for
    /// turret configurations 12/13 while leaving their palette unchanged.
    pub fn configure_enemy_map(&mut self, stage: usize, page: u8, scroll: u8) {
        if stage == 4 && page == 11 && scroll >= 192 {
            if let Some(EnemyFlight::Bullet {
                configuration,
                terrain_collision,
                ..
            }) = &mut self.enemy_state
            {
                *terrain_collision = configuration.wrapping_add(2) < 14;
            }
        }
    }
    pub fn enemy_palette(&self) -> u8 {
        match &self.enemy_state {
            Some(EnemyFlight::Bomb {
                explosion_ticks: Some(_),
                ..
            })
            | Some(EnemyFlight::Missile {
                explosion_ticks: Some(_),
                ..
            }) => 3,
            _ => self.enemy_palette,
        }
    }
    pub fn set_players(&mut self, positions: [Vec2; 2], active: [bool; 2]) {
        self.players = Some((positions, active));
    }
    pub fn actor_source_type(&self) -> Option<u8> {
        match self.enemy_state {
            Some(EnemyFlight::Bullet { .. }) => Some(54),
            Some(EnemyFlight::Grenade { .. }) => Some(52),
            Some(EnemyFlight::Bomb { .. }) => Some(55),
            Some(EnemyFlight::Missile { .. }) => Some(57),
            None if self.kind == ProjectileKind::EnemyBullet => Some(54),
            None => None,
        }
    }
    pub fn has_native_enemy_state(&self) -> bool {
        self.enemy_state.is_some()
    }
    pub fn enemy_bomb(position: Vec2, direction: u8, short_fuse: bool, global_tick: u64) -> Self {
        let mut shot = Self::enemy(position, crate::enemy::direction_velocity(direction, 0.75));
        shot.kind = ProjectileKind::EnemyGrenade;
        shot.enemy_palette = 1;
        shot.remaining = u16::MAX;
        shot.enemy_state = Some(EnemyFlight::Bomb {
            timer: if short_fuse { 48 } else { 64 },
            frame: if short_fuse { 65 } else { 64 },
            attributes: (global_tick as u8) & 0xc0,
            explosion_ticks: None,
        });
        shot
    }
    pub fn enemy_grenade(position: Vec2, direction: u8) -> Self {
        Self::enemy_grenade_pattern(position, direction, 8)
    }
    pub fn enemy_grenade_pattern(position: Vec2, direction: u8, delay: u16) -> Self {
        let direction = (((direction + 1) / 4) & 7) * 4;
        let mut shot = Self::enemy(position, crate::enemy::direction_velocity(direction, 1.));
        shot.kind = ProjectileKind::EnemyGrenade;
        shot.enemy_palette = 0;
        shot.remaining = 98 + delay;
        shot.enemy_state = Some(EnemyFlight::Grenade {
            delay,
            direction,
            frame: None,
            flips: (false, false),
        });
        shot
    }
    pub fn enemy_missile(position: Vec2) -> Self {
        Self::enemy_missile_pattern(position, 24, 2, 112)
    }
    pub fn enemy_missile_pattern(
        position: Vec2,
        direction: u8,
        variant: u8,
        base_frame: u16,
    ) -> Self {
        let variant = usize::from(variant & 3);
        let speed = [2., 1.5, 2., 3.5][variant];
        let mut shot = Self::enemy(position, crate::enemy::direction_velocity(direction, speed));
        shot.kind = ProjectileKind::EnemyMissile;
        shot.remaining = u16::MAX;
        shot.enemy_state = Some(EnemyFlight::Missile {
            direction: direction & 31,
            speed,
            maximum_turns: [1, 40, 64, 1][variant],
            base_frame,
            turn_ticks: [56, 12, 16, 0][variant],
            turn_interval: [16, 12, 6, 16][variant],
            turns: 0,
            explosion_ticks: None,
            frame: base_frame,
            flips: (false, false),
        });
        shot
    }
    /// Advance the recovered enemy projectile state machine. Returns true when
    /// a missile enters its explosion phase, for the sound event.
    pub fn tick_enemy(
        &mut self,
        target: Vec2,
        terrain: &impl crate::terrain::CollisionField,
    ) -> bool {
        let mut target = target;
        if let Some((positions, active)) = self.players {
            if let Some(EnemyFlight::Missile {
                explosion_ticks: None,
                ..
            }) = &self.enemy_state
            {
                if self.age == 0 {
                    self.target_counter = if self.actor_handle.is_some_and(|h| h.slot() & 1 != 0) {
                        128
                    } else {
                        0
                    };
                } else {
                    self.target_counter = self.target_counter.wrapping_sub(1);
                }
                if !active[usize::from(self.target_counter >> 7)] {
                    self.target_counter ^= 128;
                }
                target = positions[usize::from(self.target_counter >> 7)];
            }
        }
        self.previous = self.position;
        let mut explosion = false;
        match &mut self.enemy_state {
            Some(EnemyFlight::Bomb {
                timer,
                frame,
                attributes,
                explosion_ticks,
            }) => {
                if let Some(ticks) = explosion_ticks {
                    *ticks -= 1;
                    if *ticks == 0 {
                        self.remaining = 0;
                    } else if ticks.is_multiple_of(8) {
                        *frame += 1;
                    }
                } else if self.age > 0 {
                    *timer -= 1;
                    if *timer > 0 && timer.is_multiple_of(16) {
                        *frame += 1;
                        *attributes = attributes.wrapping_add(64);
                    }
                    self.position += self.velocity;
                    if *timer == 0 {
                        *frame = 25;
                        *explosion_ticks = Some(24);
                        self.velocity = Vec2::default();
                        explosion = true;
                    }
                }
            }
            Some(EnemyFlight::Bullet {
                direction,
                terrain_collision,
                marsh_collision,
                timer,
                travel_frame,
                delay,
                impact_ticks,
                frame,
                ..
            }) => {
                if let Some(ticks) = impact_ticks {
                    *ticks -= 1;
                    if *ticks == 0 {
                        self.remaining = 0;
                        *frame = None;
                    }
                } else if self.age > 0 {
                    self.position += self.velocity;
                    *timer -= 1;
                    let class = terrain.terrain_at(crate::enemy::infantry::lookahead(
                        self.position,
                        *direction,
                        false,
                    ));
                    let blocked = *terrain_collision
                        && *timer % 2 == 1
                        && (class == 1 || (*marsh_collision && class == 3));
                    if *timer == 0 || blocked {
                        *impact_ticks = Some(8);
                        *frame = Some(17);
                        self.velocity = Vec2::new(0., 0.);
                    } else {
                        *delay = delay.saturating_sub(1);
                        if *delay == 0 {
                            *frame = Some(*travel_frame);
                        }
                    }
                }
            }
            Some(EnemyFlight::Grenade {
                delay,
                direction,
                frame,
                flips,
            }) => {
                let offset = *delay - 8;
                let age = self.age.saturating_sub(offset);
                if self.age > 0 && age <= 72 {
                    self.position += self.velocity;
                }
                let octant = ((*direction + 1) / 4) as usize & 7;
                let attrs = [0u8, 4, 8, 0x44, 0x40, 0xc4, 0x88, 0x84][octant];
                *flips = (attrs & 0x40 != 0, attrs & 0x80 != 0);
                let base = 134 + u16::from(attrs & 15);
                *frame = match age {
                    0..=7 => None,
                    8..=23 => Some(base),
                    24..=39 => Some(base + 1),
                    40..=55 => Some(base + 2),
                    56..=71 => Some(base + 3),
                    72..=104 => Some(146),
                    _ => None,
                };
                if age >= 72 {
                    *flips = (((age.min(103) - 72) / 8) % 2 == 1, false);
                }
                // FB67/FB94 only stop on terrain class1, every other tick.
                if age > 0 && age < 56 && age % 2 == 1 && terrain.terrain_at(self.position) == 1 {
                    if age < 8 {
                        self.remaining = 0;
                    } else {
                        self.age = 56 + offset;
                        self.remaining = 50;
                        *frame = Some(base + 3);
                    }
                }
                self.remaining = self.remaining.saturating_sub(1);
            }
            Some(EnemyFlight::Missile {
                direction,
                speed,
                maximum_turns,
                base_frame,
                turn_ticks,
                turn_interval,
                turns,
                explosion_ticks,
                frame,
                flips,
            }) => {
                if let Some(timer) = explosion_ticks {
                    *timer -= 1;
                    if *timer == 0 {
                        self.remaining = 0;
                    } else if *timer == 8 {
                        *frame = 26;
                    }
                } else {
                    if self.age > 0 {
                        *turn_ticks = turn_ticks.wrapping_sub(1);
                        if *turn_ticks == 0 {
                            *turns += 1;
                            if turns.is_multiple_of(32) {
                                *turn_interval -= 1;
                            }
                            *turn_ticks = *turn_interval;
                            let wanted = crate::enemy::aim_direction(target - self.position);
                            let difference = wanted.wrapping_sub(*direction) & 31;
                            if difference != 0 {
                                *direction = if difference & 16 == 0 {
                                    direction.wrapping_add(1) & 31
                                } else {
                                    direction.wrapping_sub(1) & 31
                                };
                            }
                            self.velocity = crate::enemy::direction_velocity(*direction, *speed);
                        }
                        self.position += self.velocity;
                    }
                    let octant = ((*direction + 1) / 4) as usize & 7;
                    let attrs = [0u8, 1, 2, 0x41, 0x40, 0xc1, 0x82, 0x81][octant];
                    *frame = *base_frame + u16::from(attrs & 15);
                    *flips = (attrs & 0x40 != 0, attrs & 0x80 != 0);
                    if *turns == *maximum_turns {
                        *explosion_ticks = Some(16);
                        self.velocity = Vec2::new(0., 0.);
                        *frame = 25;
                        explosion = true;
                    }
                }
            }
            None => {
                self.tick();
                return false;
            }
        }
        self.age += 1;
        explosion
    }
    pub fn enemy_sprite(&self) -> Option<u16> {
        match &self.enemy_state {
            Some(EnemyFlight::Grenade { frame, .. }) | Some(EnemyFlight::Bullet { frame, .. }) => {
                *frame
            }
            Some(EnemyFlight::Missile { frame, .. }) | Some(EnemyFlight::Bomb { frame, .. }) => {
                Some(*frame)
            }
            None => None,
        }
    }
    pub fn sprite_flips(&self) -> (bool, bool) {
        if let Some(flips) = self.player_flips() {
            return flips;
        }
        match &self.enemy_state {
            Some(EnemyFlight::Grenade { flips, .. }) | Some(EnemyFlight::Missile { flips, .. }) => {
                *flips
            }
            Some(EnemyFlight::Bomb { attributes, .. }) => {
                (*attributes & 64 != 0, *attributes & 128 != 0)
            }
            None | Some(EnemyFlight::Bullet { .. }) => (false, false),
        }
    }
    pub fn damages_players(&self) -> bool {
        self.remaining > 0
            && match &self.enemy_state {
                Some(EnemyFlight::Grenade { delay, .. }) => self.age <= 64 + *delay,
                Some(EnemyFlight::Bullet { impact_ticks, .. }) => impact_ticks.is_none(),
                Some(EnemyFlight::Bomb {
                    explosion_ticks, ..
                }) => explosion_ticks.is_some(),
                _ => self.owner.is_none(),
            }
    }
    pub fn tick(&mut self) {
        if self.player_state.is_some() {
            self.tick_player(u64::from(self.age));
            return;
        }
        self.previous = self.position;
        self.position += self.velocity;
        self.age += 1;
        self.remaining = self.remaining.saturating_sub(1);
    }
    pub fn touches(&self, point: Vec2, radius: f32) -> bool {
        segment_hits_circle(self.previous, self.position, point, radius)
    }
    pub fn explosive(&self) -> bool {
        matches!(self.kind, ProjectileKind::Grenade | ProjectileKind::Rocket)
    }
}
#[derive(Clone, Debug)]
pub struct Explosion {
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub remaining: u16,
    pub radius: f32,
}

/// Firing follows measured input semantics: B repeats every 32 global ticks;
/// A is edge-triggered. Each jeep has three bullets and one secondary slot.
pub fn fire(
    player: &Player,
    owner: usize,
    input: PlayerInput,
    tick: u64,
    existing: &[Projectile],
) -> Vec<Projectile> {
    if !player.active() {
        return Vec::new();
    }
    let mut shots = Vec::new();
    let bullets = existing
        .iter()
        .filter(|p| p.owner == Some(owner) && p.kind == ProjectileKind::Bullet)
        .count();
    if input.gun && (!player.previous_input.gun || tick.is_multiple_of(32)) && bullets < 3 {
        shots.push(Projectile::player(
            player.position,
            Vec2::new(0., -6.),
            ProjectileKind::Bullet,
            owner,
            16,
            player.weapon,
        ));
    }
    if input.grenade
        && !player.previous_input.grenade
        && !existing
            .iter()
            .any(|p| p.owner == Some(owner) && p.explosive())
    {
        let direction = player.requested_facing.vector();
        let diagonal = direction.x != 0. && direction.y != 0.;
        let (kind, speed, lifetime) = if player.weapon == WeaponLevel::Grenade {
            (ProjectileKind::Grenade, if diagonal { 1.5 } else { 2. }, 40)
        } else {
            (ProjectileKind::Rocket, if diagonal { 2.75 } else { 4. }, 22)
        };
        shots.push(Projectile::player(
            player.position,
            direction * speed,
            kind,
            owner,
            lifetime,
            player.weapon,
        ));
    }
    shots
}

/// Central blast position. Upgrade arms are separate timed `BlastWave` actors.
pub fn blast_centers(position: Vec2, _weapon: WeaponLevel) -> Vec<Vec2> {
    vec![position]
}
