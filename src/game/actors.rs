//! Shared admission, eviction and stable slot ownership for world actors.
use super::Game;
use crate::{
    actor_pool::{ActorHandle, ActorMetadata},
    math::Vec2,
    weapon::Projectile,
};
impl Game {
    pub(super) fn allocate_actor(
        &mut self,
        source: u8,
        priority: bool,
        position: Vec2,
        event: bool,
    ) -> Option<ActorHandle> {
        let metadata = ActorMetadata::new(source, priority, self.actor_offscreen(position));
        let allocation = if event {
            self.actor_pool.allocate(metadata)
        } else {
            self.actor_pool.allocate_child(metadata)
        }?;
        if let Some(evicted) = allocation.evicted {
            self.evict_actor(evicted.handle);
        }
        Some(allocation.handle)
    }
    fn actor_offscreen(&self, position: Vec2) -> bool {
        !(self.camera.x..self.camera.x + 256.).contains(&position.x)
            || !(self.camera.y..self.camera.y + 256.).contains(&position.y)
    }
    fn evict_actor(&mut self, handle: ActorHandle) {
        macro_rules! remove {($($field:ident),*)=>{$(self.$field.retain(|a|a.actor_handle!=Some(handle));)*};}
        remove!(
            enemy_deaths,
            enemies,
            projectiles,
            explosions,
            infantry_deaths,
            prisoners,
            prison_releases,
            deliveries,
            passenger_drops,
            pickups,
            mines,
            background_objects,
            rescue_helicopters,
            rescue_zones,
            boss_children,
            environment_actors,
            current_zones
        );
        if self.pending_boss_handle == Some(handle) {
            self.pending_boss_handle = None;
        }
        if let Some(boss) = &mut self.boss {
            for part in &mut boss.parts {
                if part.actor_handle == Some(handle) {
                    part.actor_handle = None;
                    part.alive = false;
                    part.death_ticks = 0;
                }
            }
            if boss.controller_handle == Some(handle) {
                boss.controller_handle = None;
                boss.controller.alive = false;
            }
            if boss.completion_handle == Some(handle) {
                boss.completion_handle = None;
            }
            boss.completion_handles.retain(|h| *h != handle);
        }
    }
    /// Release finished actors without allowing stale generations to free a replacement.
    fn actor_locations(&self) -> Vec<(ActorHandle, Vec2)> {
        let mut live = Vec::new();
        macro_rules! record {($($field:ident),*)=>{$(for a in &self.$field {if let Some(h)=a.actor_handle {live.push((h,a.position));}})*};}
        record!(
            enemy_deaths,
            enemies,
            projectiles,
            explosions,
            infantry_deaths,
            prisoners,
            prison_releases,
            deliveries,
            passenger_drops,
            pickups,
            mines,
            background_objects,
            rescue_helicopters,
            rescue_zones,
            boss_children,
            environment_actors,
            current_zones
        );
        if let Some(h) = self.pending_boss_handle {
            live.push((h, Vec2::new(self.camera.x + 128., self.camera.y + 64.)));
        }
        if let Some(boss) = &self.boss {
            for (index, part) in boss.parts.iter().enumerate() {
                if boss.part_occupies_slot(index) {
                    if let Some(h) = part.actor_handle {
                        live.push((h, part.position.into()));
                    }
                }
            }
            for h in boss.completion_handles.iter().copied() {
                live.push((h, Vec2::new(self.camera.x + 128., self.camera.y + 64.)));
            }
            for h in [boss.controller_handle, boss.completion_handle]
                .into_iter()
                .flatten()
            {
                live.push((h, Vec2::new(self.camera.x + 128., self.camera.y + 64.)));
            }
        }
        live
    }
    pub(super) fn reconcile_actor_pool(&mut self) {
        let live = self.actor_locations();
        let old: Vec<_> = self.actor_pool.occupied().map(|(h, _)| h).collect();
        for h in old {
            if !live.iter().any(|(id, _)| *id == h) {
                self.actor_pool.release(h);
            }
        }
        for (h, p) in live {
            self.actor_pool.update_offscreen(h, self.actor_offscreen(p));
        }
    }
    pub(super) fn enqueue_projectile(&mut self, mut projectile: Projectile) {
        if projectile.has_native_player_state()
            && projectile.kind == crate::weapon::ProjectileKind::Bullet
            && projectile.player_slot.is_none()
        {
            let owner = projectile.owner.expect("player bullet owner");
            let base = 18 + owner as u8 * 3;
            let Some(slot) = (base..base + 3).rev().find(|slot| {
                !self
                    .projectiles
                    .iter()
                    .any(|shot| shot.player_slot == Some(*slot))
            }) else {
                return;
            };
            projectile.player_slot = Some(slot);
        }

        if let Some(source) = projectile.actor_source_type() {
            if projectile.actor_handle.is_none() {
                let Some(handle) = self.allocate_actor(source, false, projectile.position, false)
                else {
                    return;
                };
                projectile.actor_handle = Some(handle);
            }
        }
        self.projectiles.push(projectile);
    }
    /// Editor/test-created actors enter through the same bounded allocator.
    pub(super) fn admit_untracked_actors(&mut self) {
        for i in 0..self.enemies.len() {
            if self.enemies[i].actor_handle.is_none() {
                if let Some(h) = self.allocate_actor(
                    self.enemies[i].definition,
                    false,
                    self.enemies[i].position,
                    false,
                ) {
                    self.enemies[i].actor_handle = Some(h);
                    self.enemies[i].configure_spawn(
                        usize::from(h.slot()),
                        self.stage,
                        self.difficulty,
                    );
                } else {
                    self.enemies[i].alive = false;
                }
            }
        }
        macro_rules! admit {
            ($field:ident,$source:expr) => {{
                let mut actors = std::mem::take(&mut self.$field);
                actors.retain_mut(|actor| {
                    if actor.actor_handle.is_none() {
                        actor.actor_handle =
                            self.allocate_actor(($source)(actor), false, actor.position, false);
                    }
                    actor.actor_handle.is_some()
                });
                self.$field = actors;
            }};
        }
        admit!(prisoners, |_: &crate::rescue::Prisoner| 0x96);
        for prisoner in &mut self.prisoners {
            prisoner.set_actor_handle(prisoner.actor_handle.unwrap());
        }
        admit!(prison_releases, |a: &crate::rescue::PrisonRelease| a
            .source_type);
        admit!(deliveries, |_: &crate::rescue::Delivery| 0xa7);
        admit!(passenger_drops, |_: &crate::rescue::PassengerDrop| 40);
        admit!(mines, |_: &crate::mine::LandMine| 53);
        admit!(
            background_objects,
            |a: &crate::background_object::BackgroundObject| a.source_type
        );
        admit!(
            rescue_helicopters,
            |a: &crate::rescue_helicopter::RescueHelicopter| match a.kind {
                crate::rescue_helicopter::HelicopterKind::Flyover => 60,
                crate::rescue_helicopter::HelicopterKind::LandingLeft => 61,
                crate::rescue_helicopter::HelicopterKind::LandingRight => 62,
            }
        );
        admit!(rescue_zones, |_: &super::RescueZone| 63);
        admit!(infantry_deaths, |_: &crate::enemy::InfantryDeath| 1);
        admit!(explosions, |_: &crate::weapon::Explosion| 65);
        admit!(pickups, |a: &crate::pickup::Pickup| match a.kind {
            crate::pickup::PickupKind::Bomb => 80,
            crate::pickup::PickupKind::Weapon => 81,
            crate::pickup::PickupKind::ExtraLife => 82,
        });
        let projectiles = std::mem::take(&mut self.projectiles);
        for shot in projectiles {
            self.enqueue_projectile(shot);
        }
    }
}

impl Game {
    pub(super) fn update_enemy_slot(
        &mut self,
        slot: u8,
        assets: &crate::assets::Assets,
        target: Vec2,
    ) {
        use crate::{
            enemy::{Behavior, Enemy},
            terrain::TerrainView,
        };
        let remaining = self
            .boss
            .as_ref()
            .map_or(0, |b| b.parts.iter().filter(|p| p.alive).count() as u8);
        // F06F visits slots from 15 down to zero. Children in lower slots run
        // immediately, while children in already-visited slots wait one frame.
        let Some(index) = self
            .enemies
            .iter()
            .position(|e| e.alive && e.actor_handle.is_some_and(|h| h.slot() == slot))
        else {
            return;
        };
        let own_handle = self.enemies[index].actor_handle;
        let others = self
            .actor_locations()
            .into_iter()
            .filter(|(h, _)| Some(*h) != own_handle)
            .filter_map(|(h, p)| {
                self.actor_pool
                    .metadata(h)
                    .map(|m| (m.source_type | if m.priority { 128 } else { 0 }, p))
            })
            .collect();
        let enemy = &mut self.enemies[index];
        enemy.set_other_actors(others);
        enemy.set_players(
            self.players.each_ref().map(|p| p.position),
            self.players.each_ref().map(|p| p.active()),
        );
        enemy.set_boss_remaining(remaining);
        enemy.set_view_origin(Vec2::new(self.camera.x, self.camera.y));
        let shots = enemy.tick(
            target,
            &TerrainView {
                level: &assets.levels[self.stage],
                changes: &self.terrain,
            },
            assets,
        );
        let spawns = enemy.drain_spawns();
        self.events.extend(
            enemy
                .drain_sound_commands()
                .into_iter()
                .map(super::GameEvent::AudioCommand),
        );
        if let Some(id) = enemy.terrain_patch() {
            self.terrain.apply_boss_patch(
                &assets.levels[self.stage],
                enemy.position,
                &assets.boss_patches[usize::from(id)],
            );
        }
        if enemy.take_death_trigger() {
            self.enemy_deaths
                .push(crate::enemy::death::EnemyDeath::new(enemy, assets));
            enemy.actor_handle = None;
            enemy.alive = false;
        }
        for shot in shots {
            self.enqueue_projectile(shot);
        }
        for spawn in spawns {
            let def = assets.entity(spawn.source_type);
            if let Some(behavior) = Behavior::from_family(&def.family) {
                if let Some(handle) =
                    self.allocate_actor(spawn.source_type, false, spawn.position, false)
                {
                    let mut child = Enemy::new(def, spawn.position, behavior);
                    child.actor_handle = Some(handle);
                    child.configure_spawn(usize::from(handle.slot()), self.stage, self.difficulty);
                    child.set_spawn_tick(self.tick);
                    child.set_spawn_direction(spawn.direction, spawn.flips);
                    self.enemies.push(child);
                }
            }
        }
    }
}

impl Game {
    pub(super) fn update_enemy_death_slot(&mut self, slot: u8, assets: &crate::assets::Assets) {
        let remaining = self.boss.as_ref().map_or(0, |boss| {
            boss.parts.iter().filter(|p| p.alive).count() as u8
        });
        let page = (1.
            + (assets.levels[self.stage].start_camera_y() - self.camera.y).max(0.) / 240.)
            as u8;
        if let Some(death) = self
            .enemy_deaths
            .iter_mut()
            .find(|death| death.actor_handle.is_some_and(|h| h.slot() == slot))
        {
            self.events.extend(
                death
                    .tick(Vec2::new(self.camera.x, self.camera.y), page, remaining)
                    .into_iter()
                    .map(super::GameEvent::AudioCommand),
            );
            if !death.alive {
                if let Some(handle) = death.actor_handle.take() {
                    self.actor_pool.release(handle);
                }
            }
        }
        self.enemy_deaths.retain(|death| death.alive);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets::Assets,
        enemy::{Behavior, Enemy},
        game::GameState,
        input::Input,
        weapon::Projectile,
    };
    use std::sync::OnceLock;
    fn assets() -> &'static Assets {
        static ASSETS: OnceLock<Assets> = OnceLock::new();
        ASSETS.get_or_init(|| Assets::load(&Assets::default_directory()).unwrap())
    }
    fn session() -> Game {
        let mut game = Game::new();
        game.start_new_game(1, assets());
        game.start_stage(0, assets());
        game.enemies.clear();
        game.actor_pool.clear();
        game.state = GameState::Playing;
        game.spawn_cursor = assets().levels[0].events.len();
        game.command_cursor = assets().levels[0].commands.len();
        game
    }
    #[test]
    fn player_bullets_keep_three_independent_slots_and_reuse_the_first_free_one() {
        let mut game = session();
        let bullet = |owner| {
            Projectile::player(
                Vec2::new(100., 100.),
                Vec2::new(0., -6.),
                crate::weapon::ProjectileKind::Bullet,
                owner,
                16,
                crate::player::WeaponLevel::Grenade,
            )
        };
        for owner in 0..2 {
            for _ in 0..4 {
                game.enqueue_projectile(bullet(owner));
            }
        }
        assert_eq!(
            game.projectiles
                .iter()
                .map(|p| p.player_slot.unwrap())
                .collect::<Vec<_>>(),
            vec![20, 19, 18, 23, 22, 21]
        );
        game.projectiles.retain(|p| p.player_slot != Some(19));
        game.enqueue_projectile(bullet(0));
        assert_eq!(game.projectiles.last().unwrap().player_slot, Some(19));
        assert!(game.actor_pool.is_empty());
    }
    #[test]
    fn shared_capacity_rejects_enemy_shots_without_losing_existing_actors() {
        let mut game = session();
        let position = Vec2::new(game.camera.x + 128., game.camera.y + 64.);
        for _ in 0..20 {
            game.enemies
                .push(Enemy::new(assets().entity(1), position, Behavior::Infantry));
        }
        game.admit_untracked_actors();
        assert_eq!(game.actor_pool.len(), 16);
        assert_eq!(game.enemies.iter().filter(|e| e.alive).count(), 16);
        game.enqueue_projectile(Projectile::enemy_pattern(
            position,
            Vec2::new(0., 1.),
            128,
            0,
            16,
        ));
        assert!(game.projectiles.is_empty());
        game.update(Input::default(), assets());
        assert_eq!(game.actor_pool.len(), 16);
        assert_eq!(game.enemies.len(), 16);
    }
    #[test]
    fn infantry_death_retains_its_slot_until_animation_finishes() {
        let mut game = session();
        let position = Vec2::new(game.camera.x + 128., game.camera.y + 64.);
        game.enemies
            .push(Enemy::new(assets().entity(1), position, Behavior::Infantry));
        game.admit_untracked_actors();
        let handle = game.enemies[0].actor_handle.unwrap();
        game.damage_enemy(0, 1, false, Some(0), assets());
        assert_eq!(game.infantry_deaths[0].actor_handle, Some(handle));
        assert_eq!(game.enemies[0].actor_handle, None);
        game.reconcile_actor_pool();
        assert!(game.actor_pool.contains(handle));
        for _ in 0..40 {
            game.update(Input::default(), assets());
        }
        assert!(game.infantry_deaths.is_empty());
        assert!(!game.actor_pool.contains(handle));
    }
    #[test]
    fn priority_event_evicts_the_owned_projectile_and_stale_release_is_harmless() {
        let mut game = session();
        let position = Vec2::new(game.camera.x + 128., game.camera.y + 64.);
        for _ in 0..16 {
            game.enqueue_projectile(Projectile::enemy_pattern(
                position,
                Vec2::new(0., 1.),
                128,
                0,
                16,
            ));
        }
        let stale = game.projectiles[0].actor_handle.unwrap();
        let replacement = game.allocate_actor(1, true, position, true).unwrap();
        assert_eq!(replacement.slot(), stale.slot());
        assert_ne!(replacement, stale);
        assert_eq!(game.projectiles.len(), 15);
        assert!(!game.actor_pool.release(stale));
        assert!(game.actor_pool.contains(replacement));
    }
}
