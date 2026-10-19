use super::{Game, GameEvent};
use crate::{
    assets::Assets,
    environment::{EnvironmentActor, EnvironmentContext},
    math::Vec2,
    weapon::Projectile,
};

impl Game {
    pub(super) fn update_environment_slot(&mut self, slot: u8, assets: &Assets) {
        let camera = Vec2::new(self.camera.x, self.camera.y);
        let page = (1.
            + (assets.levels[self.stage].start_camera_y() - self.camera.y).max(0.) / 240.)
            as u8;
        let Some(index) = self.environment_actors.iter().position(|actor| {
            actor
                .actor_handle
                .is_some_and(|handle| handle.slot() == slot)
        }) else {
            return;
        };
        let mut actor = self.environment_actors.swap_remove(index);
        if actor
            .actor_handle
            .is_some_and(|handle| !self.actor_pool.contains(handle))
        {
            return;
        }
        let preferred = usize::from(slot & 1);
        let target = if self.players[preferred].active() {
            self.players[preferred].position
        } else if self.players[preferred ^ 1].active() {
            self.players[preferred ^ 1].position
        } else {
            Vec2::new(camera.x + 128., camera.y + 160.)
        };
        let context = EnvironmentContext {
            camera,
            tick: self.tick,
            page,
            forced_scroll: self.forced_scroll,
            target,
            free_slots: 16 - self.actor_pool.len(),
        };
        let events = actor.tick(&context, &mut self.formation_mask);
        if events.art_command == Some(0x10) {
            self.texture_phase_distance = 3008.;
            self.texture_phase_tick = self.tick;
        }
        for command in events.sounds {
            self.events.push(GameEvent::AudioCommand(command));
        }
        for (direction, pattern) in events.shots {
            let rank = (self.stage as u16 + u16::from(self.difficulty)) / 2;
            let mut shot = Projectile::enemy_pattern(
                actor.position,
                crate::enemy::direction_velocity(direction, 1.75),
                136 + 8 * rank,
                7,
                16,
            );
            shot.configure_enemy_terrain(self.stage, pattern);
            self.enqueue_projectile(shot);
        }
        for spawn in events.spawns {
            if let Some(handle) =
                self.allocate_actor(spawn.source_type, false, spawn.position, false)
            {
                let mut child = EnvironmentActor::new(
                    spawn.source_type,
                    spawn.position,
                    spawn.variant,
                    spawn.palette,
                )
                .unwrap();
                child.actor_handle = Some(handle);
                self.environment_actors.push(child);
            }
        }
        if actor.alive {
            self.environment_actors.push(actor);
        } else if let Some(handle) = actor.actor_handle {
            self.actor_pool.release(handle);
        }
    }
}

impl Game {
    pub(super) fn update_headquarters_effects(&mut self, assets: &Assets) {
        let Some(boss) = self.boss.as_ref() else {
            return;
        };
        let final_tick = boss.headquarters_final_death_tick();
        let center_tick = boss.headquarters_transition_tick();
        let building = boss.parts.get(3).map(|part| Vec2::from(part.position));
        let positions = if let (Some(elapsed), Some(building)) = (final_tick, building) {
            self.events.extend(
                assets
                    .headquarters_art
                    .final_sounds(elapsed)
                    .iter()
                    .copied()
                    .map(GameEvent::AudioCommand),
            );
            let mut occupied = [false; 16];
            for (handle, _) in self.actor_pool.occupied() {
                occupied[usize::from(handle.slot())] = true;
            }
            self.headquarters_explosions.final_spawn(
                elapsed,
                self.tick,
                Vec2::new(self.camera.x, self.camera.y),
                building,
                occupied,
            )
        } else if let Some(elapsed) = center_tick {
            self.events.extend(
                assets
                    .headquarters_art
                    .sounds(elapsed)
                    .iter()
                    .copied()
                    .filter(|_| elapsed != 0)
                    .map(GameEvent::AudioCommand),
            );
            self.headquarters_explosions.spawn(
                elapsed,
                self.tick,
                self.camera.y,
                16 - self.actor_pool.len(),
            )
        } else {
            Vec::new()
        };
        for position in positions {
            if let Some(handle) = self.allocate_actor(0x85, false, position, false) {
                self.explosions.push(crate::weapon::Explosion {
                    actor_handle: Some(handle),
                    position,
                    remaining: 24,
                    radius: 12.,
                });
            }
        }
    }
}

impl Game {
    /// Returns whether this projectile was consumed by an aircraft impact.
    pub(super) fn environment_projectile_hits(
        &mut self,
        shot: &mut Projectile,
        assets: &Assets,
    ) -> bool {
        if !shot.can_hit_enemies() {
            return false;
        }
        let bullet = shot.kind == crate::weapon::ProjectileKind::Bullet;
        let wave = shot.kind == crate::weapon::ProjectileKind::BlastWave;
        for actor in &mut self.environment_actors {
            if !actor.vulnerable() {
                continue;
            }
            let def = assets.entity(actor.source_type);
            let delta = shot.position - actor.position;
            if delta.x.abs() >= def.half_width || delta.y.abs() >= def.half_height {
                continue;
            }
            if actor.damage(if bullet { 1 } else { actor.health }) {
                if let Some(owner) = shot.owner {
                    self.players[owner].add_score(if self.selected_players == 2 {
                        crate::scoring::cooperative_score(def.score)
                    } else {
                        def.score
                    });
                }
            }
            if bullet || (!wave && shot.is_flying()) {
                shot.impact();
                return true;
            }
        }
        false
    }
    pub(super) fn environment_contacts(&mut self) {
        for actor in &self.environment_actors {
            if !actor.alive || !actor.harmful {
                continue;
            }
            let half = if actor.source_type == 0x38 {
                Vec2::new(12., 54.)
            } else {
                Vec2::new(10., 10.)
            };
            for player in &mut self.players {
                let delta = player.position - actor.position;
                if delta.x.abs() < half.x && delta.y.abs() < half.y && player.hit() {
                    self.events
                        .push(GameEvent::Effect(crate::sound::SoundEffect::PlayerDeath));
                }
            }
        }
    }
}

impl Game {
    pub(super) fn update_river_animation(&mut self) {
        // The final graphics stream replaces the river pattern pages.
        if self.stage != 5 || self.texture_phase_distance >= 2947. {
            self.river_phase = None;
            self.river_phase_pending = None;
            return;
        }
        if let Some((phase, deadline)) = self.river_phase_pending {
            if self.tick >= deadline {
                self.river_phase = Some(phase);
                self.river_phase_pending = None;
            }
        }
        if let Some(zone) = self
            .current_zones
            .iter()
            .filter(|zone| {
                zone.actor_handle
                    .is_none_or(|handle| self.actor_pool.contains(handle))
            })
            .min_by_key(|zone| zone.spawn_tick)
        {
            let age = self.tick.saturating_sub(zone.spawn_tick);
            if age.is_multiple_of(8) {
                self.river_phase_pending = Some((
                    crate::river_art::RiverArt::requested_phase(age),
                    self.tick + 3,
                ));
            }
        }
    }
    /// Canonical completed frame ages select the cached uploaded PNG phase.
    pub fn river_animation_frame(&self) -> Option<(bool, u64)> {
        if self.stage != 5 || self.texture_phase_distance >= 2947. {
            return None;
        }
        let after_palette = self.texture_phase_distance >= 1518.
            && self.tick.saturating_sub(self.texture_phase_tick) >= 2;
        self.river_phase
            .map(|phase| (after_palette, if phase == 0 { 3 } else { 19 }))
    }
}

impl Game {
    pub(super) fn update_rescue_zone_slot(&mut self, slot: u8, assets: &Assets) {
        let Some(index) = self.rescue_zones.iter().position(|zone| {
            zone.actor_handle
                .is_some_and(|handle| handle.slot() == slot)
        }) else {
            return;
        };
        let dock_open = self
            .rescue_helicopters
            .iter()
            .any(|helicopter| helicopter.dock_open);
        let zone = &mut self.rescue_zones[index];
        let spawn = zone.controller.tick(
            zone.position,
            Vec2::new(self.camera.x, self.camera.y),
            self.stage as u8,
            dock_open,
        );
        if !zone.controller.alive() {
            let zone = self.rescue_zones.remove(index);
            if let Some(handle) = zone.actor_handle {
                self.actor_pool.release(handle);
            }
        }
        if let Some(spawn) = spawn {
            if let Some(handle) =
                self.allocate_actor(spawn.source_type, false, spawn.position, false)
            {
                let def = assets.entity(spawn.source_type);
                let mut enemy = crate::enemy::Enemy::new(
                    def,
                    spawn.position,
                    crate::enemy::Behavior::from_family(&def.family).unwrap(),
                );
                enemy.actor_handle = Some(handle);
                enemy.set_spawn_palette(spawn.palette);
                enemy.set_spawn_tick(self.tick);
                enemy.set_view_origin(Vec2::new(self.camera.x, self.camera.y));
                enemy.configure_spawn(usize::from(handle.slot()), self.stage, self.difficulty);
                if self.selected_players == 2 {
                    enemy.score = crate::scoring::cooperative_score(enemy.score);
                }
                self.enemies.push(enemy);
            }
        }
    }
}

#[cfg(test)]
mod river_cache_tests {
    use super::*;
    #[test]
    fn uploaded_river_frame_survives_zone_expiry_and_next_zone_upload_delay() {
        let mut game = Game::new();
        game.stage = 5;
        let zone = |spawn_tick| super::super::CurrentZone {
            actor_handle: None,
            position: Vec2::ZERO,
            half: Vec2::new(12., 65.),
            spawn_tick,
        };
        game.current_zones.push(zone(100));
        for tick in 100..=119 {
            game.tick = tick;
            game.update_river_animation();
        }
        assert_eq!(game.river_animation_frame(), Some((false, 19)));
        game.current_zones.clear();
        game.tick = 149;
        game.update_river_animation();
        assert_eq!(game.river_animation_frame(), Some((false, 19)));
        game.current_zones.push(zone(150));
        for tick in 150..153 {
            game.tick = tick;
            game.update_river_animation();
            assert_eq!(game.river_animation_frame(), Some((false, 19)));
        }
        game.tick = 153;
        game.update_river_animation();
        assert_eq!(game.river_animation_frame(), Some((false, 3)));
        game.texture_phase_distance = 1518.;
        game.texture_phase_tick = 160;
        game.tick = 160;
        assert_eq!(game.river_animation_frame(), Some((false, 3)));
        game.tick = 162;
        assert_eq!(game.river_animation_frame(), Some((true, 3)));
        game.texture_phase_distance = 2947.;
        game.update_river_animation();
        assert_eq!(game.river_animation_frame(), None);
    }
}
