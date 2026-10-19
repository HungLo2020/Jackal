//! Boss controllers, parts and their children share the original descending slots.
use super::{Game, GameEvent};
use crate::{
    assets::Assets,
    boss::{Boss, BossShot, BossSpawn},
    boss_child::{BossChild, ChildEvent},
    enemy::{Behavior, Enemy},
    math::Vec2,
    terrain::TerrainView,
    weapon::{Explosion, Projectile},
};
impl Game {
    /// Award the original actor score only on the living-to-dead transition.
    pub(super) fn damage_boss(
        &mut self,
        positions: &[[f32; 2]],
        radius: f32,
        amount: u16,
        explosive: bool,
        owner: Option<usize>,
        assets: &Assets,
    ) -> bool {
        let Some(boss) = self.boss.as_mut() else {
            return false;
        };
        let alive: Vec<_> = boss.parts.iter().map(|part| part.alive).collect();
        let hit = boss.damage_area(positions, radius, amount, explosive);
        let score: u32 = boss
            .parts
            .iter()
            .zip(alive)
            .filter(|(part, was_alive)| *was_alive && !part.alive)
            .map(|(part, _)| {
                let score = assets.entity(part.source_type & 0x7f).score;
                if self.selected_players == 2 {
                    crate::scoring::cooperative_score(score)
                } else {
                    score
                }
            })
            .sum();
        if let Some(player) = owner.and_then(|owner| self.players.get_mut(owner)) {
            player.add_score(score);
        }
        hit
    }
    pub(super) fn damage_boss_child(
        &mut self,
        index: usize,
        amount: u16,
        owner: Option<usize>,
        assets: &Assets,
    ) -> bool {
        let Some(child) = self.boss_children.get_mut(index) else {
            return false;
        };
        let was_alive = child.alive && child.health > 0;
        let hit = child.damage(amount);
        if was_alive && child.health == 0 {
            let score = assets.entity(child.source_type & 0x7f).score;
            let score = if self.selected_players == 2 {
                crate::scoring::cooperative_score(score)
            } else {
                score
            };
            if let Some(player) = owner.and_then(|owner| self.players.get_mut(owner)) {
                player.add_score(score);
            }
        }
        hit
    }
    fn enqueue_boss_shot(&mut self, shot: BossShot) {
        let rank = (usize::from(self.difficulty) + self.stage) / 2;
        let mut projectile = Projectile::enemy_pattern(
            shot.position.into(),
            shot.velocity.into(),
            shot.lifetime + 8 * rank as u16,
            shot.spawn_delay,
            shot.sprite,
        );
        projectile.configure_enemy_terrain(self.stage, shot.configuration);
        self.enqueue_projectile(projectile);
    }
    fn spawn_boss_child(&mut self, spawn: BossSpawn, assets: &Assets) {
        if spawn.source_type == 57 {
            self.enqueue_projectile(Projectile::enemy_missile_pattern(
                spawn.position.into(),
                spawn.direction,
                spawn.variant,
                0x4b,
            ));
            return;
        }
        let Some(handle) =
            self.allocate_actor(spawn.source_type, false, spawn.position.into(), false)
        else {
            return;
        };
        if spawn.source_type == 7 {
            let mut enemy = Enemy::new(assets.entity(7), spawn.position.into(), Behavior::Vehicle);
            enemy.actor_handle = Some(handle);
            enemy.configure_spawn(usize::from(handle.slot()), self.stage, self.difficulty);
            enemy.set_spawn_tick(self.tick);
            enemy.score = 0;
            self.enemies.push(enemy);
        } else if let Some(mut child) = BossChild::new(spawn, handle.slot()) {
            child.set_actor_handle(handle);
            self.boss_children.push(child);
        } else {
            self.actor_pool.release(handle);
            panic!("unhandled recovered boss child {}", spawn.source_type);
        }
    }
    fn drain_boss_outputs(&mut self, boss: &mut Boss, assets: &Assets) {
        for spawn in boss.drain_spawns() {
            self.spawn_boss_child(spawn, assets);
        }
        self.events.extend(
            boss.drain_sound_commands()
                .into_iter()
                .map(GameEvent::AudioCommand),
        );
        for effect in boss.drain_explosions() {
            if let Some(handle) = self.allocate_actor(65, false, effect.position.into(), false) {
                self.explosions.push(Explosion {
                    actor_handle: Some(handle),
                    position: effect.position.into(),
                    remaining: 25,
                    radius: 12.,
                });
            }
        }
    }
    pub(super) fn begin_boss_frame(&mut self) {
        let mut boss = self.boss.take();
        if let Some(boss) = &mut boss {
            if !boss.actor_pool_enabled() {
                boss.enable_actor_pool();
            }
            if let Some(source) = boss.controller_source_type() {
                if boss.controller.alive && boss.controller_handle.is_none() {
                    boss.controller_handle = self.pending_boss_handle.take().or_else(|| {
                        self.allocate_actor(
                            source,
                            false,
                            Vec2::new(self.camera.x + 128., self.camera.y + 64.),
                            false,
                        )
                    });
                }
            } else if boss.parts[0].actor_handle.is_none() {
                if let Some(handle) = self.pending_boss_handle.take().or_else(|| {
                    self.allocate_actor(192, true, boss.parts[0].position.into(), false)
                }) {
                    boss.activate_part(0, handle);
                }
            }
            boss.set_frame_context(
                Vec2::new(self.camera.x, self.camera.y),
                self.players
                    .each_ref()
                    .map(|p| p.active().then_some(p.position)),
                self.tick,
            );
            boss.set_deployed_vehicle_count(
                self.boss_children
                    .iter()
                    .filter(|c| c.alive && matches!(c.source_type, 35 | 36))
                    .count() as u8,
            );
            boss.begin_frame();
        }
        self.boss = boss;
    }
    pub(super) fn update_boss_slot(&mut self, slot: u8, assets: &Assets, target: Vec2) {
        let mut boss = self.boss.take();

        if let Some(boss) = &mut boss {
            if boss
                .controller_handle
                .is_some_and(|h| h.slot() == slot && self.actor_pool.contains(h))
            {
                if let Some(request) = boss.tick_controller(self.tick) {
                    let handle =
                        self.allocate_actor(request.source_type, false, request.position, false);
                    boss.resolve_controller_spawn(request, handle);
                    if request.source_type == 7 {
                        if let Some(handle) = handle {
                            let mut enemy =
                                Enemy::new(assets.entity(7), request.position, Behavior::Vehicle);
                            enemy.actor_handle = Some(handle);
                            enemy.configure_spawn(
                                usize::from(handle.slot()),
                                self.stage,
                                self.difficulty,
                            );
                            enemy.set_spawn_tick(self.tick);
                            if let Some(index) = request.score_index {
                                enemy.score = crate::scoring::SCORE_VALUES[usize::from(index)];
                            }
                            self.enemies.push(enemy);
                        }
                    }
                }
                if !boss.controller.alive {
                    if let Some(handle) = boss.controller_handle.take() {
                        self.actor_pool.release(handle);
                    }
                }
            }
            if boss
                .completion_handles
                .iter()
                .any(|h| h.slot() == slot && self.actor_pool.contains(*h))
            {
                boss.tick_completion(
                    slot,
                    self.tick,
                    self.actor_pool.occupied().map(|(_, m)| m.source_type),
                );
            }
            if let Some(index) = boss.parts.iter().position(|p| {
                p.actor_handle
                    .is_some_and(|h| h.slot() == slot && self.actor_pool.contains(h))
            }) {
                let others: Vec<_> = self
                    .enemies
                    .iter()
                    .filter(|e| e.alive)
                    .map(|e| e.position)
                    .chain(
                        self.boss_children
                            .iter()
                            .filter(|c| c.alive)
                            .map(|c| c.position),
                    )
                    .collect();
                let shots = boss.tick_part(
                    index,
                    target.into(),
                    &TerrainView {
                        level: &assets.levels[self.stage],
                        changes: &self.terrain,
                    },
                    &others,
                );
                for shot in shots {
                    self.enqueue_boss_shot(shot);
                }
                self.drain_boss_outputs(boss, assets);
                if boss.kind() == crate::boss::BossKind::Headquarters && matches!(index, 0 | 3) {
                    for next in boss
                        .pending_part_indices()
                        .into_iter()
                        .filter(|&i| (index == 0 && i == 3) || (index == 3 && i == 4))
                    {
                        if let Some(handle) = self.allocate_actor(
                            boss.parts[next].source_type,
                            false,
                            boss.parts[next].position.into(),
                            false,
                        ) {
                            boss.activate_part(next, handle);
                        } else if next == 4 {
                            boss.parts[next].alive = false;
                            boss.parts[next].active = false;
                        }
                    }
                }
                if !boss.part_occupies_slot(index) {
                    if let Some(handle) = boss.parts[index].actor_handle.take() {
                        self.actor_pool.release(handle);
                    }
                }
            }
        }
        if let Some(index) = self.boss_children.iter().position(|c| {
            c.actor_handle
                .is_some_and(|h| h.slot() == slot && self.actor_pool.contains(h))
        }) {
            let others: Vec<_> = self
                .enemies
                .iter()
                .filter(|e| e.alive)
                .map(|e| e.position)
                .chain(
                    self.boss_children
                        .iter()
                        .filter(|c| c.alive)
                        .map(|c| c.position),
                )
                .collect();
            let players = std::array::from_fn(|i| {
                self.players[i].active().then_some(self.players[i].position)
            });
            let child = &mut self.boss_children[index];
            let target = child.select_target(players, target);
            child.set_camera(Vec2::new(self.camera.x, self.camera.y));
            let events = child.tick(
                target,
                self.tick,
                &TerrainView {
                    level: &assets.levels[self.stage],
                    changes: &self.terrain,
                },
                &others,
            );
            let child_position = child.position;
            let patch = child.background_patch.take();
            if let Some(patch) = patch {
                self.terrain.apply_boss_patch(
                    &assets.levels[self.stage],
                    child_position,
                    &assets.boss_patches[usize::from(patch)],
                );
            }
            for event in events {
                match event {
                    ChildEvent::Shot(shot) => self.enqueue_boss_shot(shot),
                    ChildEvent::Infantry {
                        position,
                        score_index,
                    } => {
                        if let Some(handle) = self.allocate_actor(1, false, position, false) {
                            let mut enemy =
                                Enemy::new(assets.entity(1), position, Behavior::Infantry);
                            enemy.actor_handle = Some(handle);
                            enemy.configure_spawn(
                                usize::from(handle.slot()),
                                self.stage,
                                self.difficulty,
                            );
                            enemy.set_spawn_tick(self.tick);
                            enemy.score = crate::scoring::SCORE_VALUES[usize::from(score_index)];
                            self.enemies.push(enemy);
                        }
                    }
                }
            }
            if !self.boss_children[index].alive {
                if let Some(handle) = self.boss_children[index].actor_handle.take() {
                    self.actor_pool.release(handle);
                }
            }
        }
        self.boss = boss;
    }
    pub(super) fn finish_boss_frame(&mut self, assets: &Assets) {
        let mut boss = self.boss.take();

        if let Some(boss) = &mut boss {
            boss.finish_frame();
            self.drain_boss_outputs(boss, assets);
            for part in &boss.parts {
                if let Some(id) = part.background_patch {
                    self.terrain.apply_boss_patch(
                        &assets.levels[self.stage],
                        part.position.into(),
                        &assets.boss_patches[usize::from(id)],
                    );
                }
            }
            if !boss.headquarters_guards_active() {
                for child in &mut self.boss_children {
                    if child.source_type == 72 {
                        child.alive = false;
                        if let Some(handle) = child.actor_handle.take() {
                            self.actor_pool.release(handle);
                        }
                    }
                }
            }
        }
        self.boss_children.retain(|c| c.alive);
        self.boss = boss;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actor_pool::ActorMetadata, boss::BossKind};
    use std::sync::OnceLock;
    fn assets() -> &'static Assets {
        static ASSETS: OnceLock<Assets> = OnceLock::new();
        ASSETS.get_or_init(|| Assets::load(&Assets::default_directory()).unwrap())
    }
    fn tick(game: &mut Game, frame: u64) {
        game.tick = frame;
        game.begin_boss_frame();
        for slot in (0..16).rev() {
            game.update_boss_slot(slot, assets(), Vec2::new(80., 160.));
        }
        game.finish_boss_frame(assets());
    }
    #[test]
    fn boss_scores_only_new_kills_and_respects_shields_and_armor() {
        for players in [1, 2] {
            let mut game = Game::new();
            game.selected_players = players;
            game.boss = Some(Boss::new(BossKind::FortressGuns, [256., 64.]));
            let part = &mut game.boss.as_mut().unwrap().parts[0];
            part.active = true;
            part.position = [80., 80.];
            part.shielded = true;
            assert!(!game.damage_boss(&[[80., 80.]], 0., 999, true, Some(1), assets()));
            assert_eq!(game.players[1].score, 0);
            let part = &mut game.boss.as_mut().unwrap().parts[0];
            part.shielded = false;
            part.grenade_only = true;
            assert!(!game.damage_boss(&[[80., 80.]], 0., 999, false, Some(1), assets()));
            assert_eq!(game.players[1].score, 0);
            assert!(game.damage_boss(&[[80., 80.], [80., 80.]], 0., 999, true, Some(1), assets()));
            let expected = if players == 2 { 1000 } else { 800 };
            assert_eq!(game.players[1].score, expected);
            assert_eq!(game.players[0].score, 0);
            assert!(!game.damage_boss(&[[80., 80.]], 0., 999, true, Some(1), assets()));
            assert_eq!(game.players[1].score, expected);
        }
    }
    #[test]
    fn headquarters_keeps_center_slot_until_building_spawns_and_collapse_finishes() {
        let mut game = Game::new();
        game.stage = 5;
        game.boss = Some(Boss::new(BossKind::Headquarters, [256., 64.]));
        for frame in 0..8 {
            tick(&mut game, frame);
        }
        let boss = game.boss.as_mut().unwrap();
        let center_handle = boss.parts[0].actor_handle.unwrap();
        assert!(boss.damage([256., 46.], 0., 63, true));
        for frame in 8..313 {
            tick(&mut game, frame);
        }
        let boss = game.boss.as_ref().unwrap();
        assert_eq!(boss.headquarters_transition_tick(), Some(304));
        assert!(game.actor_pool.contains(center_handle));
        assert!(boss.parts[3].actor_handle.is_none());
        tick(&mut game, 313);
        let building_handle = game.boss.as_ref().unwrap().parts[3].actor_handle.unwrap();
        assert!(building_handle.slot() > center_handle.slot());
        assert!(
            !game.boss.as_ref().unwrap().parts[3].active,
            "higher newly allocated slot must wait until next frame"
        );
        tick(&mut game, 314);
        assert!(game.boss.as_ref().unwrap().parts[3].active);
        assert!(
            game.boss.as_ref().unwrap().parts[4].active,
            "lower gun slot initializes in the same frame"
        );
        let position = game.boss.as_ref().unwrap().parts[3].position;
        assert!(game.boss.as_mut().unwrap().damage(position, 0., 127, true));
        for frame in 315..522 {
            tick(&mut game, frame);
            assert!(!game.boss.as_ref().unwrap().defeated());
        }
        tick(&mut game, 522);
        assert!(game.boss.as_ref().unwrap().defeated());
        assert_eq!(game.boss.as_ref().unwrap().parts[3].sprite, 0xd4);
    }
    #[test]
    fn full_pool_preserves_tank_controller_retry_interval() {
        let mut game = Game::new();
        game.stage = 0;
        game.boss = Some(Boss::new(BossKind::FortressGuns, [256., 64.]));
        game.boss.as_mut().unwrap().configure(0, 0);
        tick(&mut game, 0);
        assert_eq!(
            game.boss
                .as_ref()
                .unwrap()
                .controller_handle
                .unwrap()
                .slot(),
            15
        );
        let mut blockers = Vec::new();
        for _ in 0..15 {
            blockers.push(
                game.actor_pool
                    .allocate_child(ActorMetadata::new(1, false, false))
                    .unwrap()
                    .handle,
            );
        }
        for frame in 1..=240 {
            tick(&mut game, frame);
        }
        assert!(game
            .boss
            .as_ref()
            .unwrap()
            .parts
            .iter()
            .all(|p| p.actor_handle.is_none()));
        assert!(game.actor_pool.release(blockers[0]));
        for frame in 241..480 {
            tick(&mut game, frame);
        }
        assert!(game.boss.as_ref().unwrap().parts[0].actor_handle.is_none());
        tick(&mut game, 480);
        let part = &game.boss.as_ref().unwrap().parts[0];
        assert_eq!(part.actor_handle.unwrap().slot(), 14);
        assert_eq!(part.position, [352., 239.]);
        assert_eq!(game.actor_pool.len(), 16);
    }
    #[test]
    fn every_boss_uses_shared_slots_without_overbooking() {
        for stage in 0..6 {
            let mut game = Game::new();
            game.stage = stage;
            game.boss = Some(Boss::new(BossKind::for_stage(stage), [256., 64.]));
            game.boss.as_mut().unwrap().configure(2, 0);
            for frame in 0..1200 {
                tick(&mut game, frame);
                let handles: Vec<_> = game.actor_pool.occupied().map(|(h, _)| h).collect();
                assert!(handles.len() <= 16, "stage{stage} frame{frame}");
                let boss = game.boss.as_ref().unwrap();
                for part in &boss.parts {
                    if part.active && part.alive {
                        assert!(
                            part.actor_handle
                                .is_some_and(|h| game.actor_pool.contains(h)),
                            "stage{stage} frame{frame} unallocated activepart"
                        );
                    }
                }
                for child in &game.boss_children {
                    assert!(
                        child
                            .actor_handle
                            .is_some_and(|h| game.actor_pool.contains(h)),
                        "stage{stage} frame{frame} unallocated child"
                    );
                }
            }
        }
    }
}
