//! Session state and system ordering. Gameplay is ordinary Rust data; rendering
//! and device audio consume snapshots/events and never advance the simulation.
mod actors;
mod bosses;
mod environment;
use crate::terrain::{CollisionField, TerrainChanges, TerrainView};
use crate::{
    assets::{Assets, Level},
    boss::{Boss, BossKind},
    boss_child::BossChild,
    camera::CameraController,
    enemy::{Behavior, Enemy, InfantryDeath},
    input::{Input, PlayerInput},
    math::Vec2,
    player::{Player, WeaponLevel, RESPAWN_PROTECTION, STARTING_LIVES},
    renderer::Camera,
    rescue::{Delivery, PassengerDrop, PrisonRelease},
    rescue_helicopter::{HelicopterContext, RescueHelicopter},
    sound::{MusicTrack, SoundEffect},
    weapon::{self, Explosion, Projectile, ProjectileKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Title,
    Attract { ticks: u16 },
    Briefing { ticks: u16 },
    Playing,
    StageLoading { ticks: u16 },
    StageClear { ticks: u16 },
    GameOver { ticks: u16 },
    Continue { ticks: u16 },
    Ending { ticks: u16 },
    EndingRestart { ticks: u16 },
}
#[derive(Clone, Copy, Debug)]
pub enum GameEvent {
    Music(MusicTrack),
    AudioCommand(u8),
    Effect(SoundEffect),
    Pause(bool),
}
pub use crate::pickup::{Pickup, PickupKind};
pub use crate::rescue::Prisoner;
#[derive(Clone, Debug)]
pub struct RescueZone {
    pub controller: crate::rescue_helicopter::RescueZoneController,
    pub actor_handle: Option<crate::actor_pool::ActorHandle>,
    pub position: Vec2,
    pub cooldown: u16,
}

struct CurrentZone {
    actor_handle: Option<crate::actor_pool::ActorHandle>,
    position: Vec2,
    half: Vec2,
    spawn_tick: u64,
}

pub struct Game {
    actor_pool: crate::actor_pool::ActorPool,
    pub state: GameState,
    pub stage: usize,
    pub tick: u64,
    pub sprite_rotation: u8,
    pub selected_players: usize,
    pub camera: Camera,
    pub players: [Player; 2],
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub explosions: Vec<Explosion>,
    pub infantry_deaths: Vec<InfantryDeath>,
    pub enemy_deaths: Vec<crate::enemy::death::EnemyDeath>,
    pub prisoners: Vec<Prisoner>,
    pub prison_releases: Vec<PrisonRelease>,
    pub deliveries: Vec<Delivery>,
    next_prisoner_id: u64,
    passenger_drops: Vec<PassengerDrop>,
    unload_ticks: [u16; 2],
    pub pickups: Vec<Pickup>,
    pub mines: Vec<crate::mine::LandMine>,
    current_zones: Vec<CurrentZone>,
    river_phase: Option<u8>,
    river_phase_pending: Option<(u8, u64)>,
    pub environment_actors: Vec<crate::environment::EnvironmentActor>,
    formation_mask: u8,
    pending_boss_handle: Option<crate::actor_pool::ActorHandle>,
    pub background_objects: Vec<crate::background_object::BackgroundObject>,
    pub terrain: TerrainChanges,
    pub rescue_zones: Vec<RescueZone>,
    pub rescue_helicopters: Vec<RescueHelicopter>,
    pub boss: Option<Boss>,
    pub boss_children: Vec<BossChild>,
    camera_controller: CameraController,
    pub paused: bool,
    pub continues_used: u8,
    pub continue_selected: bool,
    pub title_ticks: u16,
    pub tally_delay: u16,
    pub loading_continue: bool,
    pub texture_phase_tick: u64,
    pub texture_phase_distance: f32,
    pub difficulty: u8,
    hard_loop: bool,
    headquarters_explosions: crate::hq_effects::CollapseExplosions,
    spawn_cursor: usize,
    command_cursor: usize,
    forced_scroll: bool,
    previous_input: Input,
    events: Vec<GameEvent>,
    boss_spawned: bool,
}
impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
impl Game {
    pub fn new() -> Self {
        Self {
            actor_pool: crate::actor_pool::ActorPool::default(),
            state: GameState::Title,
            stage: 0,
            tick: 0,
            sprite_rotation: 0,
            selected_players: 1,
            camera: Camera::default(),
            players: [
                Player::new(Vec2::ZERO, true),
                Player::new(Vec2::ZERO, false),
            ],
            enemies: Vec::new(),
            projectiles: Vec::new(),
            explosions: Vec::new(),
            infantry_deaths: Vec::new(),
            enemy_deaths: Vec::new(),
            prisoners: Vec::new(),
            prison_releases: Vec::new(),
            deliveries: Vec::new(),
            next_prisoner_id: 1,
            passenger_drops: Vec::new(),
            unload_ticks: [16; 2],
            pickups: Vec::new(),
            mines: Vec::new(),
            current_zones: Vec::new(),
            river_phase: None,
            river_phase_pending: None,
            environment_actors: Vec::new(),
            formation_mask: 255,
            pending_boss_handle: None,
            background_objects: Vec::new(),
            terrain: TerrainChanges::default(),
            rescue_zones: Vec::new(),
            rescue_helicopters: Vec::new(),
            boss: None,
            boss_children: Vec::new(),
            camera_controller: CameraController::default(),
            paused: false,
            continues_used: 0,
            continue_selected: true,
            title_ticks: 0,
            tally_delay: 0,
            loading_continue: false,
            texture_phase_tick: 0,
            texture_phase_distance: 0.,
            difficulty: 0,
            hard_loop: false,
            headquarters_explosions: crate::hq_effects::CollapseExplosions::default(),
            spawn_cursor: 0,
            command_cursor: 0,
            forced_scroll: false,
            previous_input: Input::default(),
            events: vec![GameEvent::Music(MusicTrack::Title)],
            boss_spawned: false,
        }
    }
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
    pub fn start_new_game(&mut self, players: usize, assets: &Assets) {
        self.selected_players = players.clamp(1, 2);
        self.continues_used = 0;
        self.difficulty = 0;
        self.hard_loop = false;
        self.paused = false;
        self.previous_input = Input::default();
        self.events.push(GameEvent::Pause(false));
        self.players = [
            Player::new(Vec2::ZERO, true),
            Player::new(Vec2::ZERO, self.selected_players == 2),
        ];
        self.start_stage(0, assets);
    }
    /// Stage loading is also used by the editor/headless verifier. It has no
    /// address-based state injection and validates the requested campaign index.
    pub fn start_stage(&mut self, stage: usize, assets: &Assets) {
        assert!(stage < assets.levels.len(), "stage index outside campaign");
        self.actor_pool.clear();
        self.stage = stage;
        self.loading_continue = false;
        self.texture_phase_tick = self.tick;
        self.texture_phase_distance = 0.;
        let level = &assets.levels[stage];
        self.camera = Camera {
            x: level.start_camera_x,
            y: level.start_camera_y(),
        };
        for (i, player) in self.players.iter_mut().enumerate() {
            player.position = Vec2::new(self.camera.x + 80. + i as f32 * 96., self.camera.y + 128.);
            player.invulnerable = RESPAWN_PROTECTION;
            player.respawn_ticks = 0;
            player.previous_input = PlayerInput::default();
            player.river_current = false;
        }
        self.terrain.clear();
        self.enemies.clear();
        self.projectiles.clear();
        self.explosions.clear();
        self.infantry_deaths.clear();
        self.enemy_deaths.clear();
        self.prisoners.clear();
        self.prison_releases.clear();
        self.deliveries.clear();
        self.passenger_drops.clear();
        self.unload_ticks = [16; 2];
        self.pickups.clear();
        self.mines.clear();
        self.current_zones.clear();
        self.river_phase = None;
        self.river_phase_pending = None;
        self.background_objects.clear();
        self.environment_actors.clear();
        self.formation_mask = 255;
        self.pending_boss_handle = None;
        self.rescue_zones.clear();
        self.rescue_helicopters.clear();
        self.boss = None;
        self.headquarters_explosions = crate::hq_effects::CollapseExplosions::default();
        self.boss_children.clear();
        self.camera_controller.reset();
        self.spawn_cursor = 0;
        self.command_cursor = 0;
        self.forced_scroll = false;
        self.boss_spawned = false;
        self.state = GameState::Playing;
        self.events
            .push(GameEvent::Music(MusicTrack::Stage(stage as u8)));
        self.spawn_events(assets);
    }
    fn begin_stage_loading(&mut self, stage: usize, continued: bool, assets: &Assets) {
        self.start_stage(stage, assets);
        // Music begins when loading finishes, not over the preceding scene.
        if matches!(
            self.events.last(),
            Some(GameEvent::Music(MusicTrack::Stage(_)))
        ) {
            self.events.pop();
        }
        self.loading_continue = continued;
        self.state = GameState::StageLoading { ticks: 0 };
    }
    pub fn update(&mut self, input: Input, assets: &Assets) {
        let start = input.start && !self.previous_input.start;
        let select = input.select && !self.previous_input.select;
        if input.pause && !self.previous_input.pause && self.state == GameState::Playing {
            self.paused = !self.paused;
            self.events.push(GameEvent::Pause(self.paused));
        }
        if self.paused {
            self.previous_input = input;
            return;
        }
        let previous_state = self.state;
        self.tick += 1;
        self.sprite_rotation = self.sprite_rotation.wrapping_add(17) & 63;
        match self.state {
            GameState::Title => {
                if select {
                    self.selected_players = 3 - self.selected_players;
                }
                if start {
                    self.state = GameState::Briefing { ticks: 0 };
                } else {
                    self.title_ticks = self.title_ticks.saturating_add(1);
                    if self.title_ticks >= 385 {
                        self.state = GameState::Attract { ticks: 0 };
                    }
                }
            }
            GameState::Attract { ticks } => {
                if start {
                    self.state = GameState::Briefing { ticks: 0 };
                } else if u32::from(ticks) + 1
                    >= assets
                        .cinematics
                        .get("attract")
                        .expect("attract asset")
                        .duration_ticks()
                {
                    self.return_to_title();
                } else {
                    self.state = GameState::Attract { ticks: ticks + 1 };
                }
            }
            GameState::Briefing { ticks } => {
                if ticks == 81 {
                    self.sprite_rotation = 0;
                }
                let name = if self.selected_players == 2 {
                    "intro_2p"
                } else {
                    "intro_1p"
                };
                if u32::from(ticks) + 1
                    >= assets
                        .cinematics
                        .get(name)
                        .expect("intro asset")
                        .duration_ticks()
                {
                    self.start_new_game(self.selected_players, assets);
                } else {
                    self.state = GameState::Briefing { ticks: ticks + 1 };
                }
            }
            GameState::StageLoading { ticks } => {
                let prefix = if self.loading_continue {
                    "continue_load"
                } else {
                    "stage_load"
                };
                let clip = assets
                    .cinematics
                    .get(&format!("{prefix}_{:02}", self.stage + 1))
                    .expect("stage loading asset");
                if u32::from(ticks) + 1 >= clip.duration_ticks() {
                    self.state = GameState::Playing;
                    self.events
                        .push(GameEvent::Music(MusicTrack::Stage(self.stage as u8)));
                } else {
                    self.state = GameState::StageLoading { ticks: ticks + 1 };
                }
            }
            GameState::Playing => self.play(input, assets),
            GameState::StageClear { ticks } => {
                let had_prisoners = self.players.iter().any(|p| p.delivered > 0);
                if ticks >= 211 && self.tick.is_multiple_of(8) {
                    for player in &mut self.players {
                        if player.delivered > 0 {
                            player.delivered -= 1;
                            player.add_score(2000);
                        }
                    }
                }
                let has_prisoners = self.players.iter().any(|p| p.delivered > 0);
                let hold = if ticks == 419 {
                    if had_prisoners && !has_prisoners {
                        self.tally_delay = 96;
                    }
                    if has_prisoners {
                        true
                    } else if self.tally_delay > 0 {
                        self.tally_delay -= 1;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                let name = format!("stage_clear_{:02}", self.stage + 1);
                if u32::from(ticks) + 1
                    >= assets
                        .cinematics
                        .get(&name)
                        .expect("stage clear asset")
                        .duration_ticks()
                {
                    self.begin_stage_loading(self.stage + 1, false, assets);
                } else if !hold {
                    self.state = GameState::StageClear { ticks: ticks + 1 };
                }
            }
            GameState::GameOver { ticks } => {
                if ticks == 0 {
                    if self.continues_used < 3 {
                        self.continue_selected = true;
                        self.state = GameState::Continue { ticks: 0 };
                    } else {
                        self.return_to_title();
                    }
                } else {
                    self.state = GameState::GameOver { ticks: ticks - 1 };
                }
            }
            GameState::Continue { ticks } => {
                if select {
                    self.continue_selected = !self.continue_selected;
                }
                if start && !self.continue_selected {
                    self.return_to_title();
                } else if start {
                    self.continues_used += 1;
                    for p in &mut self.players {
                        if p.enabled {
                            p.lives = STARTING_LIVES;
                            p.weapon = WeaponLevel::Grenade;
                            p.passengers = 0;
                        }
                    }
                    self.begin_stage_loading(self.stage, true, assets);
                } else {
                    self.state = GameState::Continue {
                        ticks: ticks.saturating_add(1),
                    };
                }
            }
            GameState::EndingRestart { ticks } => {
                if u32::from(ticks) + 1
                    >= assets
                        .cinematics
                        .get("ending_restart")
                        .expect("ending restart asset")
                        .duration_ticks()
                {
                    self.difficulty = 3;
                    self.hard_loop = true;
                    self.continues_used = 3;
                    self.begin_stage_loading(0, false, assets);
                } else {
                    self.state = GameState::EndingRestart { ticks: ticks + 1 };
                }
            }
            GameState::Ending { ticks } => {
                if input.start
                    && u32::from(ticks) + 1
                        >= assets
                            .cinematics
                            .get("ending")
                            .expect("ending asset")
                            .duration_ticks()
                {
                    self.state = GameState::EndingRestart { ticks: 0 };
                } else {
                    self.state = GameState::Ending {
                        ticks: ticks.saturating_add(1),
                    };
                }
            }
        }
        if self.state != previous_state {
            if let Some((clip, tick)) = self.presentation(assets) {
                self.events
                    .extend(clip.sound_commands_at(tick).map(GameEvent::AudioCommand));
            }
        }
        self.previous_input = input;
    }
    pub fn presentation<'a>(
        &self,
        assets: &'a Assets,
    ) -> Option<(&'a crate::cinematic::Cinematic, u32)> {
        let (name, tick) = match self.state {
            GameState::Briefing { ticks } => (
                if self.selected_players == 2 {
                    "intro_2p"
                } else {
                    "intro_1p"
                },
                u32::from(ticks),
            ),
            GameState::Attract { ticks } => ("attract", u32::from(ticks)),
            GameState::Continue { ticks } => (
                if self.continue_selected {
                    "continue_yes"
                } else {
                    "continue_no"
                },
                u32::from(ticks),
            ),
            GameState::Ending { ticks } => ("ending", u32::from(ticks)),
            GameState::EndingRestart { ticks } => ("ending_restart", u32::from(ticks)),
            GameState::GameOver { ticks } if self.continues_used >= 3 => (
                "game_over",
                assets
                    .cinematics
                    .get("game_over")?
                    .duration_ticks()
                    .saturating_sub(u32::from(ticks) + 1),
            ),
            GameState::StageLoading { ticks } if self.loading_continue => (
                [
                    "continue_load_01",
                    "continue_load_02",
                    "continue_load_03",
                    "continue_load_04",
                    "continue_load_05",
                    "continue_load_06",
                ][self.stage],
                u32::from(ticks),
            ),
            GameState::StageLoading { ticks } => (
                [
                    "stage_load_01",
                    "stage_load_02",
                    "stage_load_03",
                    "stage_load_04",
                    "stage_load_05",
                    "stage_load_06",
                ][self.stage],
                u32::from(ticks),
            ),
            GameState::StageClear { ticks } => (
                [
                    "stage_clear_01",
                    "stage_clear_02",
                    "stage_clear_03",
                    "stage_clear_04",
                    "stage_clear_05",
                ]
                .get(self.stage)
                .copied()?,
                u32::from(ticks),
            ),
            _ => return None,
        };
        Some((
            assets
                .cinematics
                .get(name)
                .expect("required presentation asset"),
            tick,
        ))
    }
    fn return_to_title(&mut self) {
        self.state = GameState::Title;
        self.title_ticks = 0;
        self.events.push(GameEvent::Music(MusicTrack::Title));
    }
    fn play(&mut self, input: Input, assets: &Assets) {
        self.reconcile_actor_pool();
        self.admit_untracked_actors();
        for explosion in &mut self.explosions {
            explosion.remaining = explosion.remaining.saturating_sub(1);
        }
        self.explosions.retain(|e| e.remaining > 0);
        for object in &mut self.background_objects {
            if object.tick() {
                self.pickups.push(Pickup::new(
                    object.position,
                    PickupKind::Weapon,
                    self.stage,
                    self.selected_players == 2,
                ));
            }
        }
        self.background_objects
            .retain(|o| o.position.y < self.camera.y + 320.);
        let carried_before = self
            .players
            .each_ref()
            .map(|p| if p.active() { p.passengers } else { 0 });
        for death in &mut self.infantry_deaths {
            death.age += 1;
        }
        self.infantry_deaths
            .retain(|death| death.frame(assets).is_some());
        let level = &assets.levels[self.stage];
        for i in 0..2 {
            if self.forced_scroll {
                continue;
            }
            self.players[i].tick(
                input.players[i],
                &TerrainView {
                    level,
                    changes: &self.terrain,
                },
                self.stage,
                self.tick,
            );
            let mut shots = weapon::fire(
                &self.players[i],
                i,
                input.players[i],
                self.tick,
                &self.projectiles,
            );
            for shot in &mut shots {
                if self.stage == 5 && self.boss.is_some() {
                    shot.set_flight_ticks(if shot.kind == ProjectileKind::Grenade {
                        50
                    } else {
                        32
                    });
                }
                self.events
                    .push(GameEvent::Effect(if shot.kind == ProjectileKind::Bullet {
                        SoundEffect::MachineGun
                    } else if shot.kind == ProjectileKind::Rocket {
                        SoundEffect::Rocket
                    } else {
                        SoundEffect::Grenade
                    }));
            }
            for shot in shots {
                self.enqueue_projectile(shot);
            }
            self.players[i].previous_input = input.players[i];
            self.players[i].river_current = false;
        }
        self.update_camera(level);
        self.spawn_events(assets);
        let target = self
            .players
            .iter()
            .find(|p| p.active())
            .map(|p| p.position)
            .unwrap_or(Vec2::new(self.camera.x + 128., self.camera.y + 160.));
        self.begin_boss_frame();
        for slot in (0..16).rev() {
            self.update_enemy_death_slot(slot, assets);
            self.update_enemy_slot(slot, assets, target);
            self.update_boss_slot(slot, assets, target);
            self.update_environment_slot(slot, assets);
            self.update_projectile_slot(slot, assets);
            self.update_rescue_zone_slot(slot, assets);
        }
        self.finish_boss_frame(assets);
        self.update_headquarters_effects(assets);
        let positions =
            self.players
                .each_ref()
                .map(|p| if p.active() { Some(p.position) } else { None });
        for mine in &mut self.mines {
            mine.tick(positions);
        }
        self.mines
            .retain(|m| m.alive && m.position.y < self.camera.y + 320.);
        for pickup in &mut self.pickups {
            pickup.tick();
        }
        self.update_projectiles(assets);
        self.update_rescue(assets);
        self.collect_and_rescue(assets);
        self.contact_damage(assets);
        self.environment_contacts();
        for (i, carried) in carried_before.into_iter().enumerate() {
            if carried > 1 && self.players[i].respawn_ticks == crate::player::DEATH_TICKS {
                if let Some(handle) =
                    self.allocate_actor(40, false, self.players[i].position, false)
                {
                    let mut drop = PassengerDrop::new(self.players[i].position, carried);
                    drop.actor_handle = Some(handle);
                    self.passenger_drops.push(drop);
                }
            }
        }
        self.enemies.retain(|e| {
            e.alive
                && e.position.y < self.camera.y + 320.
                && e.position.x > -64.
                && e.position.x < level.width() + 64.
        });
        self.prisoners
            .retain(|p| p.position.y < self.camera.y + 280.);
        self.pickups.retain(|p| p.position.y < self.camera.y + 280.);
        self.update_river_animation();
        self.current_zones
            .retain(|z| z.position.y < self.camera.y + 320.);
        self.reconcile_actor_pool();
        if self.boss.as_ref().is_some_and(|b| b.defeated()) {
            self.projectiles.clear();
            self.tally_delay = 0;
            if self.stage == 5 {
                self.state = GameState::Ending { ticks: 0 };
            } else {
                self.state = GameState::StageClear { ticks: 0 };
            }
        } else if self.players.iter().all(|p| !p.enabled || p.lives == 0) {
            self.state = GameState::GameOver {
                ticks: if self.continues_used < 3 {
                    0
                } else {
                    assets
                        .cinematics
                        .get("game_over")
                        .expect("game over asset")
                        .duration_ticks() as u16
                        - 1
                },
            };
            self.events.push(GameEvent::Music(MusicTrack::GameOver));
        }
    }
    fn update_camera(&mut self, level: &Level) {
        self.camera_controller.update(
            &mut self.camera,
            &mut self.players,
            level,
            self.boss.is_some(),
            self.forced_scroll,
        );
    }
    fn spawn_events(&mut self, assets: &Assets) {
        let level = &assets.levels[self.stage];
        let distance = 240. + level.start_camera_y() - self.camera.y;
        while self.command_cursor < level.commands.len()
            && level.commands[self.command_cursor].distance <= distance
        {
            if level.commands[self.command_cursor].command == "forced_scroll" {
                self.forced_scroll = true;
            } else {
                self.texture_phase_tick = self.tick;
                self.texture_phase_distance = level.commands[self.command_cursor].distance;
            }
            self.command_cursor += 1;
        }
        while self.spawn_cursor < level.events.len()
            && level.events[self.spawn_cursor].distance <= distance
        {
            let event = &level.events[self.spawn_cursor];
            self.spawn_cursor += 1;
            if event.odd_difficulty_only && self.difficulty < 3 {
                continue;
            }
            let position = Vec2::new(
                event.x,
                level.start_camera_y() - (event.distance - 240.)
                    + if event.from_bottom { 224. } else { 0. },
            );
            let Some(handle) =
                self.allocate_actor(event.source_type, event.priority_spawn, position, true)
            else {
                // F0DB discards the rest of this same-distance spawn group.
                while self.spawn_cursor < level.events.len()
                    && level.events[self.spawn_cursor].distance == event.distance
                {
                    self.spawn_cursor += 1;
                }
                continue;
            };
            let def = assets.entity(event.source_type);
            match event.source_type {
                27 | 78 | 83 => {
                    let mut object =
                        crate::background_object::BackgroundObject::new(position, def, self.stage);
                    object.actor_handle = Some(handle);
                    if self.selected_players == 2 {
                        object.score = crate::scoring::cooperative_score(object.score);
                    }
                    self.background_objects.push(object);
                }
                68 => self.current_zones.push(CurrentZone {
                    actor_handle: Some(handle),
                    position,
                    spawn_tick: self.tick,
                    half: Vec2::new(12., if event.distance >= 2400. { 89. } else { 65. }),
                }),
                53 => {
                    let mut mine = crate::mine::LandMine::new(position);
                    mine.actor_handle = Some(handle);
                    self.mines.push(mine);
                }
                50 | 56 | 73 => {
                    let mut actor = crate::environment::EnvironmentActor::new(
                        event.source_type,
                        position,
                        0,
                        0,
                    )
                    .unwrap();
                    actor.actor_handle = Some(handle);
                    self.environment_actors.push(actor);
                }
                _ => match def.family.as_str() {
                    "boss" => {
                        self.pending_boss_handle = Some(handle);
                    }
                    "prisoner" => {
                        let mut prisoner = Prisoner::waiting(position, false);
                        prisoner.set_actor_handle(handle);
                        self.prisoners.push(prisoner);
                    }
                    "pickup" => {
                        if !self.hard_loop {
                            let mut pickup = Pickup::new(
                                position,
                                match def.kind.as_str() {
                                    "bomb_pickup" => PickupKind::Bomb,
                                    "weapon_pickup" => PickupKind::Weapon,
                                    _ => PickupKind::ExtraLife,
                                },
                                self.stage,
                                self.selected_players == 2,
                            );
                            pickup.actor_handle = Some(handle);
                            self.pickups.push(pickup);
                        } else {
                            self.actor_pool.release(handle);
                        }
                    }
                    "helicopter" => {
                        if let Some(mut heli) = RescueHelicopter::new(event.source_type, position) {
                            heli.actor_handle = Some(handle);
                            self.rescue_helicopters.push(heli);
                        }
                    }
                    "rescue_zone" => self.rescue_zones.push(RescueZone {
                        actor_handle: Some(handle),
                        position,
                        cooldown: 0,
                        controller: Default::default(),
                    }),
                    "effect" => {
                        self.actor_pool.release(handle);
                    }
                    family => {
                        if let Some(behavior) = Behavior::from_family(family) {
                            let mut enemy = Enemy::new(def, position, behavior);
                            enemy.actor_handle = Some(handle);
                            enemy.set_spawn_tick(self.tick);
                            if self.selected_players == 2 {
                                enemy.score = crate::scoring::cooperative_score(enemy.score);
                            }
                            enemy.configure_spawn(
                                usize::from(handle.slot()),
                                self.stage,
                                self.difficulty,
                            );
                            self.enemies.push(enemy);
                        } else {
                            self.actor_pool.release(handle);
                        }
                    }
                },
            }
        }
        if !self.boss_spawned && distance >= level.boss.distance {
            self.boss_spawned = true;
            self.forced_scroll = false;
            self.boss = Some(Boss::new(
                BossKind::for_stage(self.stage),
                [self.camera.x + 128., self.camera.y + 64.],
            ));
            self.boss
                .as_mut()
                .unwrap()
                .configure(self.difficulty, self.tick);
            self.events.push(GameEvent::Music(MusicTrack::Boss));
        }
    }
    fn update_projectiles(&mut self, assets: &Assets) {
        let projectiles = std::mem::take(&mut self.projectiles);
        for shot in projectiles {
            if shot.actor_handle.is_some() {
                self.projectiles.push(shot);
            } else {
                self.update_projectile(shot, assets);
            }
        }
    }
    fn update_projectile_slot(&mut self, slot: u8, assets: &Assets) {
        if let Some(index) = self
            .projectiles
            .iter()
            .position(|shot| shot.actor_handle.is_some_and(|h| h.slot() == slot))
        {
            let shot = self.projectiles.remove(index);
            self.update_projectile(shot, assets);
        }
    }
    fn update_projectile(&mut self, mut shot: Projectile, assets: &Assets) {
        let level = &assets.levels[self.stage];
        if shot.has_native_player_state() {
            self.update_player_projectile(&mut shot, assets);
            if shot.remaining > 0
                && shot.position.y >= self.camera.y - 64.
                && shot.position.y < self.camera.y + 320.
                && shot.position.x >= 0.
                && shot.position.x < level.width()
            {
                self.projectiles.push(shot);
            }
            return;
        }
        let enemy_actor = shot.has_native_enemy_state();
        if enemy_actor {
            let scroll = (level.start_camera_y() - self.camera.y).max(0.) as u32;
            if shot.age == 0 {
                shot.configure_enemy_map(
                    self.stage,
                    (1 + scroll / 240) as u8,
                    (scroll % 240) as u8,
                );
            }
            shot.set_players(
                self.players.each_ref().map(|p| p.position),
                self.players.each_ref().map(|p| p.active()),
            );
            let target = self
                .players
                .iter()
                .find(|p| p.active())
                .map(|p| p.position)
                .unwrap_or(shot.position);
            if shot.tick_enemy(
                target,
                &TerrainView {
                    level,
                    changes: &self.terrain,
                },
            ) {
                self.events.push(GameEvent::Effect(SoundEffect::Explosion));
            }
        } else {
            shot.tick();
        }
        let mut consumed = false;
        if shot.kind == ProjectileKind::EnemyBullet || enemy_actor {
            for player in &mut self.players {
                if (!enemy_actor || shot.damages_players())
                    && shot.touches(player.position, 7.)
                    && player.hit()
                {
                    self.events
                        .push(GameEvent::Effect(SoundEffect::PlayerDeath));
                    consumed = shot.kind == ProjectileKind::EnemyBullet;
                    break;
                }
            }
        } else if shot.kind != ProjectileKind::Grenade {
            for idx in 0..self.enemies.len() {
                let e = &self.enemies[idx];
                let def = assets.entity(e.definition);
                if e.alive
                    && e.collision_half_size(def).is_some_and(|half| {
                        let d = shot.position - e.position;
                        d.x.abs() < half.x && d.y.abs() < half.y
                    })
                {
                    if shot.explosive() {
                        self.explode(&shot, assets);
                    } else {
                        self.damage_enemy(idx, 1, false, shot.owner, assets);
                    }
                    consumed = true;
                    break;
                }
            }
            if !consumed {
                if shot.kind == ProjectileKind::Bullet {
                    consumed =
                        self.damage_boss(&[shot.position.into()], 0., 1, false, shot.owner, assets);
                } else if self.boss.as_ref().is_some_and(|boss| {
                    boss.parts.iter().any(|p| {
                        p.active
                            && p.alive
                            && shot.position.distance(p.position.into()) < p.radius + 4.
                    })
                }) {
                    self.explode(&shot, assets);
                    consumed = true;
                }
            }
            let terrain = TerrainView {
                level,
                changes: &self.terrain,
            }
            .terrain_at(shot.position);
            if !consumed && (terrain == 1 || terrain == 3) && self.stage != 5 {
                if shot.explosive() {
                    self.explode(&shot, assets);
                }
                consumed = true;
            }
        }
        if !consumed && shot.remaining == 0 {
            if shot.explosive() {
                self.explode(&shot, assets);
            }
            consumed = true;
        }
        if !consumed
            && shot.position.y >= self.camera.y - if enemy_actor { 64. } else { 96. }
            && shot.position.y < self.camera.y + if enemy_actor { 320. } else { 336. }
            && shot.position.x >= 0.
            && shot.position.x < level.width()
        {
            self.projectiles.push(shot);
        } else if let Some(handle) = shot.actor_handle {
            self.actor_pool.release(handle);
        }
    }
    fn update_player_projectile(&mut self, shot: &mut Projectile, assets: &Assets) {
        if shot.tick_player(self.tick) {
            self.events.push(GameEvent::Effect(SoundEffect::Explosion));
            self.projectiles.extend(shot.blast_waves());
        }
        if !shot.can_hit_enemies() {
            return;
        }
        if self.environment_projectile_hits(shot, assets) {
            return;
        }
        for object in &mut self.background_objects {
            let delta = shot.position - object.position;
            if delta.x.abs() < object.half.x
                && delta.y.abs() < object.half.y
                && object.hit(if shot.kind == ProjectileKind::Bullet {
                    1
                } else {
                    object.health
                })
            {
                if object.health == 0 {
                    self.terrain.apply_boss_patch(
                        &assets.levels[self.stage],
                        object.position,
                        &assets.boss_patches[usize::from(object.patch)],
                    );
                    if let Some(sound) = object.sound {
                        self.events.push(GameEvent::AudioCommand(sound));
                    }
                    if let Some(owner) = shot.owner {
                        self.players[owner].add_score(object.score);
                    }
                }
                shot.impact();
                return;
            }
        }
        for mine in &mut self.mines {
            if shot.touches(mine.position, 10.) && mine.detonate() {
                self.events.push(GameEvent::AudioCommand(0x17));
                shot.impact();
                return;
            }
        }
        for pickup in &mut self.pickups {
            if shot.touches(pickup.position, 10.) && pickup.reveal() {
                if let Some(owner) = shot.owner {
                    self.players[owner].add_score(pickup.score);
                }
                shot.impact();
                return;
            }
        }
        let bullet = shot.kind == ProjectileKind::Bullet;
        let wave = shot.kind == ProjectileKind::BlastWave;
        for index in 0..self.enemies.len() {
            let enemy = &self.enemies[index];
            let def = assets.entity(enemy.definition);
            let Some(half) = enemy.collision_half_size(def) else {
                continue;
            };
            let delta = shot.position - enemy.position;
            if !enemy.alive || delta.x.abs() >= half.x || delta.y.abs() >= half.y {
                continue;
            }
            if def.grenade_only && (bullet || wave || !shot.is_flying()) {
                continue;
            }
            let passes = def.secondary_passes_through;
            let damage = if bullet {
                1
            } else if def.grenade_only {
                5
            } else {
                enemy.health
            };
            self.damage_enemy(index, damage, !bullet, shot.owner, assets);
            if bullet || (!wave && shot.is_flying() && !passes) {
                shot.impact();
                break;
            }
        }
        if shot.can_hit_enemies() && !bullet && !wave && shot.is_flying() {
            for index in 0..self.boss_children.len() {
                let child = &self.boss_children[index];
                let definition = assets.entity(child.source_type & 127);
                let delta = shot.position - child.position;
                if delta.x.abs() < definition.half_width
                    && delta.y.abs() < definition.half_height
                    && self.damage_boss_child(index, 5, shot.owner, assets)
                {
                    shot.impact();
                    break;
                }
            }
        }
        if shot.can_hit_enemies()
            && (bullet || (!wave && shot.is_flying()))
            && self.damage_boss(
                &[shot.position.into()],
                0.,
                if bullet { 1 } else { 5 },
                !bullet,
                shot.owner,
                assets,
            )
        {
            shot.impact();
        }
        if shot.is_flying() && shot.kind != ProjectileKind::Grenade {
            let terrain = TerrainView {
                level: &assets.levels[self.stage],
                changes: &self.terrain,
            }
            .terrain_at(shot.position);
            if matches!(terrain, 1 | 3) && self.stage != 5 {
                shot.impact();
            }
        }
    }
    fn explode(&mut self, shot: &Projectile, assets: &Assets) {
        let centers = weapon::blast_centers(shot.position, shot.upgrade);
        self.events.push(GameEvent::Effect(SoundEffect::Explosion));
        for idx in 0..self.enemies.len() {
            if self.enemies[idx].alive
                && centers
                    .iter()
                    .any(|&p| p.distance(self.enemies[idx].position) <= 22.)
            {
                self.damage_enemy(idx, 5, true, shot.owner, assets);
            }
        }
        let positions: Vec<[f32; 2]> = centers.iter().map(|&center| center.into()).collect();
        self.damage_boss(&positions, 16., 5, true, shot.owner, assets);
        self.explosions
            .extend(centers.into_iter().map(|position| Explosion {
                actor_handle: None,
                position,
                remaining: 24,
                radius: 16.,
            }));
    }
    fn damage_enemy(
        &mut self,
        index: usize,
        amount: u16,
        explosive: bool,
        owner: Option<usize>,
        assets: &Assets,
    ) {
        let e = &mut self.enemies[index];
        let def = assets.entity(e.definition);
        if !e.damage(amount, explosive, def) {
            return;
        }
        if e.alive {
            if !explosive {
                self.events.push(GameEvent::AudioCommand(0x10));
            }
            return;
        }
        let death = crate::enemy::death::EnemyDeath::new(e, assets);
        let position = e.position;
        let actor_handle = e.actor_handle.take();
        let prison = e.behavior == Behavior::Prison;
        let source_type = e.definition;
        if let Some(id) = e.terrain_patch() {
            self.terrain.apply_boss_patch(
                &assets.levels[self.stage],
                position,
                &assets.boss_patches[usize::from(id)],
            );
        }
        if let Some(patch) = assets.destruction(self.stage, e.definition) {
            self.terrain
                .destroy_building(&assets.levels[self.stage], position, patch);
        }
        if let Some(owner) = owner {
            self.players[owner].add_score(e.score);
        }
        if e.behavior == Behavior::Infantry {
            self.infantry_deaths.push(InfantryDeath {
                actor_handle,
                position,
                age: 0,
            });
            self.events
                .push(GameEvent::Effect(SoundEffect::InfantryDeath));
        } else if !prison {
            self.enemy_deaths.push(death);
        }
        if prison {
            let mut release = PrisonRelease::new(position, source_type);
            release.actor_handle = actor_handle;
            self.prison_releases.push(release);
            self.events.push(GameEvent::AudioCommand(0x1b));
        }
    }
    fn update_rescue(&mut self, assets: &Assets) {
        let context = HelicopterContext {
            camera: Vec2::new(self.camera.x, self.camera.y),
            tick: self.tick,
            carried_prisoners: self.players.iter().map(|p| u16::from(p.passengers)).sum(),
            forced_scroll: self.forced_scroll,
        };
        for heli in &mut self.rescue_helicopters {
            self.events
                .extend(heli.tick(&context).into_iter().map(GameEvent::AudioCommand));
        }
        self.rescue_helicopters.retain(|h| h.alive());
        for mut drop in std::mem::take(&mut self.passenger_drops) {
            if !drop.finished() {
                if let Some(handle) = self.allocate_actor(0x96, false, drop.position, false) {
                    if let Some(mut prisoner) = drop.next(self.tick, self.next_prisoner_id) {
                        self.next_prisoner_id += 1;
                        prisoner.set_actor_handle(handle);
                        self.prisoners.push(prisoner);
                    }
                }
            }
            if !drop.finished() {
                self.passenger_drops.push(drop);
            } else if let Some(h) = drop.actor_handle {
                self.actor_pool.release(h);
            }
        }
        let presence: Vec<_> = self.prisoners.iter().map(|p| (p.id, p.emerged())).collect();
        for mut release in std::mem::take(&mut self.prison_releases) {
            let ready = release
                .previous
                .is_none_or(|id| presence.iter().find(|p| p.0 == id).is_none_or(|p| p.1));
            // A full actor pool postpones a release rather than losing a prisoner.
            let capacity = self.actor_pool.len() < 16;
            if release.tick(ready && capacity) {
                if let Some(handle) = self.allocate_actor(0x96, false, release.position, false) {
                    let id = self.next_prisoner_id;
                    self.next_prisoner_id += 1;
                    let mut prisoner = release.prisoner(id);
                    prisoner.set_actor_handle(handle);
                    self.prisoners.push(prisoner);
                    release.previous = Some(id);
                }
            }
            if release.source_type == 25 && release.age == 25 {
                if let Some(handle) = self.allocate_actor(35, false, release.position, false) {
                    if let Some(mut child) = BossChild::new(
                        crate::boss::BossSpawn {
                            source_type: 35,
                            position: release.position.into(),
                            direction: 0,
                            variant: 0,
                        },
                        handle.slot(),
                    ) {
                        child.set_actor_handle(handle);
                        self.boss_children.push(child);
                    }
                }
            }
            // F90B keeps the ruined building's actor until it leaves the viewport.
            if release.position.y < self.camera.y + 320. {
                self.prison_releases.push(release);
            } else if let Some(h) = release.actor_handle {
                self.actor_pool.release(h);
            }
        }
        let presence: Vec<_> = self.prisoners.iter().map(|p| p.id).collect();
        for prisoner in &mut self.prisoners {
            let previous = prisoner.previous.is_some_and(|id| presence.contains(&id));
            prisoner.tick(
                self.tick,
                previous,
                &TerrainView {
                    level: &assets.levels[self.stage],
                    changes: &self.terrain,
                },
            );
        }
        for delivery in &mut self.deliveries {
            if delivery.tick() {
                self.players[delivery.owner].add_score(500);
                self.events.push(GameEvent::AudioCommand(0x1f));
            }
        }
        self.deliveries.retain(|d| d.remaining > 0);
    }
    fn collect_and_rescue(&mut self, assets: &Assets) {
        for i in 0..self.players.len() {
            if !self.players[i].active() {
                continue;
            }
            let mut index = 0;
            while index < self.prisoners.len() {
                if self.prisoners[index].collectible()
                    && self.players[i]
                        .position
                        .distance(self.prisoners[index].position)
                        < 12.
                    && self.players[i].rescue(self.prisoners[index].special)
                {
                    let special = self.prisoners[index].special;
                    self.prisoners.remove(index);
                    if special {
                        self.difficulty =
                            (self.players[0].weapon as u8 + self.players[1].weapon as u8).min(3);
                        self.events.push(GameEvent::AudioCommand(0x21));
                    }
                    self.events.push(GameEvent::Effect(SoundEffect::Rescue));
                } else {
                    index += 1;
                }
            }
            let mut index = 0;
            while index < self.pickups.len() {
                if self.pickups[index].visible()
                    && self.players[i]
                        .position
                        .distance(self.pickups[index].position)
                        < 14.
                {
                    let pickup = self.pickups.remove(index);
                    match pickup.kind {
                        PickupKind::Bomb => {
                            for enemy in &mut self.enemies {
                                if enemy.position.y >= self.camera.y
                                    && enemy.position.y < self.camera.y + 240.
                                    && !matches!(
                                        enemy.behavior,
                                        Behavior::Environment | Behavior::Prison
                                    )
                                {
                                    enemy.alive = false;
                                    if enemy.behavior == Behavior::Infantry {
                                        self.infantry_deaths.push(InfantryDeath {
                                            actor_handle: enemy.actor_handle.take(),
                                            position: enemy.position,
                                            age: 0,
                                        });
                                        self.events
                                            .push(GameEvent::Effect(SoundEffect::InfantryDeath));
                                    } else {
                                        self.enemy_deaths.push(
                                            crate::enemy::death::EnemyDeath::new(enemy, assets),
                                        );
                                        enemy.actor_handle = None;
                                    }
                                }
                            }
                        }
                        PickupKind::Weapon => {
                            self.players[i].weapon = WeaponLevel::FourWay;
                            self.difficulty = 3;
                        }
                        PickupKind::ExtraLife => {
                            self.players[i].lives = (self.players[i].lives + 1).min(9)
                        }
                    }
                    self.events.push(GameEvent::Effect(SoundEffect::Pickup));
                } else {
                    index += 1;
                }
            }
        }
        for owner in 0..2 {
            let player = &self.players[owner];
            if !player.active() || player.passengers == 0 {
                continue;
            }
            let Some(zone) = self
                .rescue_zones
                .iter()
                .find(|z| player.position.distance(z.position) < 28.)
            else {
                continue;
            };
            let position = player.position;
            let left = self
                .rescue_helicopters
                .iter()
                .find_map(|h| h.unloading_direction())
                .map_or(zone.position.x < position.x, |d| d > 0);
            self.unload_ticks[owner] = self.unload_ticks[owner].saturating_sub(1);
            if self.unload_ticks[owner] == 0 {
                self.unload_ticks[owner] = 32;
                if let Some(handle) = self.allocate_actor(0xa7, false, position, false) {
                    if self.players[owner].deliver_one() {
                        self.deliveries.push(Delivery {
                            actor_handle: Some(handle),
                            position,
                            owner,
                            remaining: 96,
                            last: self.players[owner].passengers == 0,
                            palette: 0,
                            left,
                        });
                    } else {
                        self.actor_pool.release(handle);
                    }
                }
            }
        }
    }
    fn contact_damage(&mut self, assets: &Assets) {
        for player in &mut self.players {
            if !player.active() {
                continue;
            }
            player.river_current = self.current_zones.iter().any(|zone| {
                let delta = zone.position - player.position;
                delta.x.abs() < zone.half.x && delta.y.abs() < zone.half.y
            });
            for mine in &mut self.mines {
                let delta = mine.position - player.position;
                if mine.armed() && delta.x.abs() < 10. && delta.y.abs() < 10. && player.hit() {
                    mine.detonate();
                    self.events.push(GameEvent::AudioCommand(0x17));
                    self.events
                        .push(GameEvent::Effect(SoundEffect::PlayerDeath));
                }
            }
            for child in &self.boss_children {
                if let Some(half) = child.harmful_half_extents() {
                    let d = child.position - player.position;
                    if d.x.abs() < half.x && d.y.abs() < half.y && player.hit() {
                        self.events
                            .push(GameEvent::Effect(SoundEffect::PlayerDeath));
                    }
                }
            }
            for enemy in &mut self.enemies {
                let Some(half) = enemy.collision_half_size(assets.entity(enemy.definition)) else {
                    continue;
                };
                let delta = enemy.position - player.position;
                if !enemy.alive || delta.x.abs() >= half.x || delta.y.abs() >= half.y {
                    continue;
                }
                if enemy.behavior == Behavior::Infantry {
                    enemy.alive = false;
                    player.add_score(enemy.score);
                    self.infantry_deaths.push(InfantryDeath {
                        actor_handle: enemy.actor_handle.take(),
                        position: enemy.position,
                        age: 0,
                    });
                    self.events
                        .push(GameEvent::Effect(SoundEffect::InfantryDeath));
                } else if matches!(
                    enemy.behavior,
                    Behavior::Vehicle | Behavior::Boat | Behavior::Turret
                ) && player.hit()
                {
                    self.events
                        .push(GameEvent::Effect(SoundEffect::PlayerDeath));
                }
            }
        }
    }
}
