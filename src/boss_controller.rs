//! Native encounter controllers: their spawn retries are part of gameplay.
use crate::{boss::BossKind, math::Vec2};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerSpawn {
    pub source_type: u8,
    pub position: Vec2,
    pub variant: u8,
    pub part_index: Option<usize>,
    pub score_index: Option<u8>,
}
#[derive(Clone, Debug)]
pub struct BossController {
    kind: BossKind,
    difficulty: u8,
    pub phase: u8,
    pub alive: bool,
    count: u8,
    timer: u8,
    interval: u8,
    reinforcement_timer: u8,
    reinforcement_count: u8,
    pending: Option<ControllerSpawn>,
    camera_y: f32,
}
impl BossController {
    pub fn new(kind: BossKind, difficulty: u8, camera_y: f32) -> Self {
        Self {
            kind,
            difficulty,
            phase: 0,
            alive: kind != BossKind::AttackAircraft,
            count: 0,
            timer: 0,
            interval: 0,
            reinforcement_timer: 0,
            reinforcement_count: 0,
            pending: None,
            camera_y,
        }
    }
    fn request(
        &mut self,
        source_type: u8,
        x: f32,
        y: f32,
        variant: u8,
        part_index: Option<usize>,
        score_index: Option<u8>,
    ) -> Option<ControllerSpawn> {
        let request = ControllerSpawn {
            source_type,
            position: Vec2::new(x, self.camera_y + y),
            variant,
            part_index,
            score_index,
        };
        self.pending = Some(request);
        Some(request)
    }
    pub fn tick(&mut self, clock: u8) -> Option<ControllerSpawn> {
        assert!(
            self.pending.is_none(),
            "resolve the previous controller spawn before advancing"
        );
        if !self.alive {
            return None;
        }
        if self.phase == 0 {
            self.count = match self.kind {
                BossKind::StatueBattery => 6,
                BossKind::Headquarters => 2,
                _ => 4,
            };
            self.interval = if self.kind == BossKind::FortressGuns {
                [240, 208, 176, 144][usize::from(self.difficulty.min(3))]
            } else {
                24
            };
            self.timer = self.interval;
            self.phase = 1;
            return None;
        }
        match self.kind {
            BossKind::FortressGuns | BossKind::BattleshipGuns | BossKind::StatueBattery => {
                if self.phase == 1 {
                    self.timer = self.timer.wrapping_sub(1);
                    if self.timer != 0 {
                        return None;
                    }
                    self.timer = self.interval;
                    self.count = self.count.wrapping_sub(1);
                    if self.count == 255 {
                        return self.request(70, 0., 0., 0, None, None);
                    }
                    let i = usize::from(self.count);
                    match self.kind {
                        BossKind::FortressGuns => self.request(
                            10,
                            f32::from(clock.clamp(80, 176)) * 2.,
                            if self.count & 1 != 0 { 239. } else { 0. },
                            0,
                            Some(3 - i),
                            None,
                        ),
                        BossKind::BattleshipGuns => {
                            self.request(24, [188., 268., 228., 308.][i], 28., 0, Some(i), None)
                        }
                        BossKind::StatueBattery => self.request(
                            146,
                            [112., 184., 184., 232., 232., 296.][i],
                            [70., 54., 86., 54., 86., 70.][i],
                            1,
                            Some(i),
                            None,
                        ),
                        _ => unreachable!(),
                    }
                } else if self.phase == 2 && self.kind != BossKind::FortressGuns {
                    if self.difficulty < 2 {
                        if self.kind == BossKind::StatueBattery {
                            return self.request(70, 0., 0., 0, None, None);
                        }
                        self.phase = 3;
                        return None;
                    }
                    self.reinforcement_timer = self.reinforcement_timer.wrapping_sub(1);
                    if self.reinforcement_timer != 0 {
                        return None;
                    }
                    self.reinforcement_count = self.reinforcement_count.wrapping_add(1);
                    if self.reinforcement_count & 1 != 0 {
                        return None;
                    }
                    let (low, high) = if self.kind == BossKind::StatueBattery {
                        (64, 192)
                    } else {
                        (80, 176)
                    };
                    self.request(
                        7,
                        f32::from(clock.clamp(low, high)) * 2.,
                        239.,
                        u8::from(self.kind == BossKind::StatueBattery),
                        None,
                        if self.kind == BossKind::BattleshipGuns {
                            Some(0)
                        } else {
                            None
                        },
                    )
                } else {
                    self.alive = false;
                    None
                }
            }
            BossKind::HeavyTank => match self.phase {
                1 => self.request(38, 280., 64., 0, Some(4), None),
                2 => {
                    self.count = self.count.wrapping_sub(1);
                    if self.count == 255 {
                        self.request(70, 0., 0., 0, None, None)
                    } else {
                        let i = usize::from(self.count);
                        self.request(
                            49,
                            [128., 176., 384., 432.][i],
                            72.,
                            self.count,
                            Some(i),
                            None,
                        )
                    }
                }
                _ => {
                    self.alive = false;
                    None
                }
            },
            BossKind::Headquarters => match self.phase {
                1 => self.request(74, 256., 46., 0, Some(0), None),
                2 => {
                    self.count = self.count.wrapping_sub(1);
                    if self.count == 255 {
                        self.phase = 3;
                        None
                    } else {
                        self.request(
                            71,
                            if self.count == 1 { 302. } else { 210. },
                            41.,
                            0,
                            Some(if self.count == 1 { 1 } else { 2 }),
                            None,
                        )
                    }
                }
                _ => {
                    self.alive = false;
                    None
                }
            },
            BossKind::AttackAircraft => None,
        }
    }
    pub fn resolve_spawn(&mut self, success: bool) {
        let spawn = self.pending.take().expect("no pending controller spawn");
        if spawn.source_type == 7 {
            return;
        }
        if success {
            if spawn.source_type == 70
                || matches!(
                    (self.kind, self.phase),
                    (BossKind::HeavyTank | BossKind::Headquarters, 1)
                )
            {
                self.phase += 1;
            }
        } else if self.phase == 1
            && matches!(
                self.kind,
                BossKind::FortressGuns | BossKind::BattleshipGuns | BossKind::StatueBattery
            )
        {
            self.count = self.count.wrapping_add(1);
            if spawn.source_type == 70 {
                self.timer = if self.kind == BossKind::FortressGuns {
                    self.timer.wrapping_add(1)
                } else {
                    1
                };
            }
        } else if self.phase == 2
            && matches!(self.kind, BossKind::HeavyTank | BossKind::Headquarters)
        {
            self.count = self.count.wrapping_add(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controller_spawns_and_pool_failure_retries_match_original() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/boss_controllers.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let stage = capture["stage"].as_u64().unwrap() as usize;
            let difficulty = capture["difficulty"].as_u64().unwrap() as u8;
            let blocked = capture["blocked"].as_bool().unwrap();
            let mut controller = BossController::new(BossKind::for_stage(stage), difficulty, 0.);
            for sample in capture["samples"].as_array().unwrap() {
                let tick = sample["tick"].as_u64().unwrap();
                let spawn = controller.tick(tick as u8);
                let success = !sample["full"].as_bool().unwrap();
                if spawn.is_some() {
                    controller.resolve_spawn(success);
                }
                let emitted = if success { spawn } else { None };
                let expected = sample["spawns"].as_array().unwrap();
                assert_eq!(
                    usize::from(emitted.is_some()),
                    expected.len(),
                    "stage{stage} rank{difficulty} blocked{blocked} tick{tick} count"
                );
                if let Some(spawn) = emitted {
                    assert_eq!(
                        u64::from(spawn.source_type),
                        expected[0]["source_type"].as_u64().unwrap(),
                        "stage{stage} rank{difficulty} blocked{blocked} tick{tick} type"
                    );
                    if spawn.source_type != 70 {
                        assert_eq!(
                            f64::from(spawn.position.x),
                            expected[0]["x"].as_f64().unwrap(),
                            "stage{stage} rank{difficulty} blocked{blocked} tick{tick} x"
                        );
                        assert_eq!(
                            f64::from(spawn.position.y),
                            expected[0]["y"].as_f64().unwrap(),
                            "stage{stage} rank{difficulty} blocked{blocked} tick{tick} y"
                        );
                        assert_eq!(
                            u64::from(spawn.variant),
                            expected[0]["variant"].as_u64().unwrap(),
                            "stage{stage} rank{difficulty} blocked{blocked} tick{tick} variant"
                        );
                    }
                }
                assert_eq!(
                    controller.alive,
                    sample["alive"].as_bool().unwrap(),
                    "stage{stage} rank{difficulty} blocked{blocked} tick{tick} alive"
                );
                if controller.alive {
                    assert_eq!(
                        u64::from(controller.phase),
                        sample["phase"].as_u64().unwrap(),
                        "stage{stage} rank{difficulty} blocked{blocked} tick{tick} phase"
                    );
                }
            }
        }
    }
}
