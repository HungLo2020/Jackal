//! Campaign integration checks. Acceleration changes only named world/session
//! state; encounter victory still goes through its public damage API.
use jackal_native::{
    assets::Assets,
    boss::BossKind,
    enemy::Behavior,
    game::{Game, GameState, Prisoner, RescueZone},
    input::{Input, PlayerInput},
    math::Vec2,
    player::WeaponLevel,
    weapon::Projectile,
};
use std::sync::OnceLock;
fn assets() -> &'static Assets {
    static A: OnceLock<Assets> = OnceLock::new();
    A.get_or_init(|| Assets::load(&Assets::default_directory()).unwrap())
}
fn step(g: &mut Game, input: Input) {
    g.update(input, assets());
}
fn pulse_start(g: &mut Game) {
    step(g, Input::default());
    step(
        g,
        Input {
            start: true,
            ..Default::default()
        },
    );
}
fn protect_and_center(g: &mut Game) {
    for (i, p) in g.players.iter_mut().enumerate() {
        if p.enabled {
            p.invulnerable = 1000;
            p.position = Vec2::new(g.camera.x + 80. + i as f32 * 96., g.camera.y + 160.);
        }
    }
}
/// Headless traversal accelerator preserves original event ordering while avoiding
/// a dependence on authored enemy aim or on keyboard-driven route finding.
fn advance_to_encounter(g: &mut Game) -> usize {
    let l = &assets().levels[g.stage];
    let target = l.start_camera_y() - (l.boss.distance - 240.);
    let mut observed = std::collections::HashSet::new();
    for _ in 0..1200 {
        for e in &g.enemies {
            observed.insert(e.definition);
        }
        if g.boss.is_some() {
            return observed.len();
        }
        g.camera.y = (g.camera.y - 4.).max(target);
        protect_and_center(g);
        step(g, Input::default());
        assert_eq!(
            g.state,
            GameState::Playing,
            "traversal unexpectedly left gameplay"
        );
    }
    panic!("stage {} failed to start its encounter", g.stage + 1)
}
fn defeat_encounter(g: &mut Game) {
    for _ in 0..2000 {
        if matches!(
            g.state,
            GameState::StageClear { .. } | GameState::Ending { .. }
        ) {
            return;
        }
        let boss = g.boss.as_mut().expect("encounter loaded");
        let targets: Vec<_> = boss
            .parts
            .iter()
            .filter(|p| p.alive && p.active && !p.shielded)
            .map(|p| p.position)
            .collect();
        for position in targets {
            boss.damage(position, 1., 5, true);
        }
        protect_and_center(g);
        step(g, Input::default());
    }
    panic!(
        "stage {} encounter cannot be defeated through damage",
        g.stage + 1
    )
}
#[test]
fn title_select_two_players_briefing_and_independent_controls() {
    let mut g = Game::new();
    assert_eq!(g.state, GameState::Title);
    step(
        &mut g,
        Input {
            select: true,
            ..Default::default()
        },
    );
    assert_eq!(g.selected_players, 2);
    step(
        &mut g,
        Input {
            select: true,
            ..Default::default()
        },
    );
    assert_eq!(
        g.selected_players, 2,
        "held select must not toggle each tick"
    );
    pulse_start(&mut g);
    assert!(matches!(g.state, GameState::Briefing { .. }));
    pulse_start(&mut g);
    assert!(
        matches!(g.state, GameState::Briefing { .. }),
        "Start must not skip the helicopter"
    );
    while matches!(g.state, GameState::Briefing { .. }) {
        step(&mut g, Input::default());
    }
    assert_eq!(g.state, GameState::Playing);
    assert!(g.players.iter().all(|p| p.active() && p.lives == 5));
    let y = g.camera.y + 128.;
    g.players[0].position = Vec2::new(80., y);
    g.players[1].position = Vec2::new(96., y);
    let before = [g.players[0].position, g.players[1].position];
    step(
        &mut g,
        Input {
            players: [
                PlayerInput {
                    right: true,
                    ..Default::default()
                },
                PlayerInput {
                    left: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    );
    assert_eq!(g.players[0].position.x, before[0].x + 1.);
    assert_eq!(g.players[1].position.x, before[1].x - 1.);
}
#[test]
fn pause_freezes_world_clock_positions_and_projectiles_until_new_edge() {
    let mut g = Game::new();
    g.start_new_game(1, assets());
    let origin = g.players[0].position;
    g.projectiles.push(Projectile::enemy(
        origin + Vec2::new(40., 0.),
        Vec2::new(1., 0.),
    ));
    step(
        &mut g,
        Input {
            pause: true,
            ..Default::default()
        },
    );
    let clock = g.tick;
    let bullet = g.projectiles[0].position;
    for _ in 0..30 {
        step(
            &mut g,
            Input {
                pause: true,
                players: [
                    PlayerInput {
                        right: true,
                        gun: true,
                        ..Default::default()
                    },
                    PlayerInput::default(),
                ],
                ..Default::default()
            },
        );
    }
    assert!(g.paused);
    assert_eq!(g.tick, clock);
    assert_eq!(g.players[0].position, origin);
    assert_eq!(g.projectiles[0].position, bullet);
    step(&mut g, Input::default());
    assert_eq!(g.tick, clock);
    step(
        &mut g,
        Input {
            pause: true,
            ..Default::default()
        },
    );
    assert!(!g.paused);
    assert_eq!(g.tick, clock + 1);
    assert_ne!(g.projectiles[0].position, bullet);
}
#[test]
fn all_event_families_have_an_explicit_native_route() {
    let a = assets();
    for level in &a.levels {
        for event in &level.events {
            let def = a.entity(event.source_type);
            let special = matches!(
                def.family.as_str(),
                "boss" | "prisoner" | "pickup" | "rescue_zone" | "effect"
            );
            assert!(
                special || Behavior::from_family(&def.family).is_some(),
                "{} silently drops {} family {}",
                level.name,
                def.kind,
                def.family
            );
            assert!(
                !def.kind.starts_with("entity_"),
                "unclassified event {}",
                def.source_type
            );
        }
    }
}
#[test]
fn all_six_stage_encounters_clear_in_sequence_and_ending_restarts_harder_loop() {
    let mut g = Game::new();
    g.start_new_game(2, assets());
    for stage in 0..6 {
        assert_eq!(g.stage, stage);
        assert_eq!(g.state, GameState::Playing);
        let observed = advance_to_encounter(&mut g);
        assert!(
            observed >= 5,
            "stage {} too few event families spawned: {observed}",
            stage + 1
        );
        assert_eq!(g.boss.as_ref().unwrap().kind(), BossKind::for_stage(stage));
        defeat_encounter(&mut g);
        if stage < 5 {
            assert!(matches!(g.state, GameState::StageClear { .. }));
            for _ in 0..601 {
                step(&mut g, Input::default());
            }
        }
    }
    assert!(matches!(g.state, GameState::Ending { .. }));
    let score = g.players[0].score;
    for _ in 0..6500 {
        step(&mut g, Input::default());
    }
    assert!(
        matches!(g.state, GameState::Ending { .. }),
        "ending holds until Start"
    );
    pulse_start(&mut g);
    assert!(matches!(g.state, GameState::EndingRestart { .. }));
    for _ in 0..116 {
        step(&mut g, Input::default());
    }
    assert_eq!(g.stage, 0);
    assert_eq!(g.state, GameState::Playing);
    assert_eq!(g.difficulty, 3);
    assert_eq!(g.players[0].score, score);
    assert!(g.players[1].enabled);
}
#[test]
fn three_continues_restore_selected_players_and_fourth_game_over_returns_to_title() {
    let mut g = Game::new();
    g.start_new_game(2, assets());
    g.start_stage(2, assets());
    for used in 0..=3 {
        for p in &mut g.players {
            p.lives = 0;
            p.weapon = WeaponLevel::FourWay;
            p.passengers = 3;
        }
        step(&mut g, Input::default());
        assert!(matches!(g.state, GameState::GameOver { .. }));
        for _ in 0..if used < 3 { 1 } else { 450 } {
            step(&mut g, Input::default());
        }
        if used < 3 {
            assert!(matches!(g.state, GameState::Continue { .. }));
            pulse_start(&mut g);
            assert!(matches!(g.state, GameState::StageLoading { .. }));
            for _ in 0..assets()
                .cinematics
                .get("continue_load_03")
                .unwrap()
                .duration_ticks()
            {
                step(&mut g, Input::default());
            }
            assert_eq!(g.continues_used, used + 1);
            assert_eq!(g.stage, 2);
            assert_eq!(g.state, GameState::Playing);
            assert!(g
                .players
                .iter()
                .all(|p| p.lives == 5 && p.weapon == WeaponLevel::Grenade && p.passengers == 0));
        } else {
            assert_eq!(g.state, GameState::Title);
        }
    }
}
#[test]
fn prisoner_pickup_delivery_and_death_use_gameplay_systems() {
    let mut g = Game::new();
    g.start_new_game(1, assets());
    let position = g.players[0].position;
    g.prisoners.push(Prisoner::waiting(position, true));
    step(&mut g, Input::default());
    assert!(g.prisoners.is_empty());
    assert_eq!(g.players[0].passengers, 1);
    assert_eq!(g.players[0].weapon, WeaponLevel::Rocket);
    for _ in 0..3 {
        g.prisoners.push(Prisoner::waiting(position, false));
    }
    step(&mut g, Input::default());
    assert_eq!(g.players[0].passengers, 4);
    g.rescue_helicopters
        .push(jackal_native::rescue_helicopter::RescueHelicopter::new(0x3d, position).unwrap());
    g.rescue_zones.push(RescueZone {
        actor_handle: None,
        controller: Default::default(),
        position,
        cooldown: 0,
    });
    for _ in 0..15 {
        step(&mut g, Input::default());
    }
    assert_eq!(g.players[0].delivered, 0);
    step(&mut g, Input::default());
    assert_eq!(g.players[0].delivered, 1);
    for _ in 0..31 {
        step(&mut g, Input::default());
    }
    assert_eq!(g.players[0].delivered, 1, "delivery repeated before32ticks");
    for _ in 0..65 {
        step(&mut g, Input::default());
    }
    assert_eq!(g.players[0].delivered, 4);
    assert_eq!(g.players[0].passengers, 0);
    assert_eq!(g.players[0].weapon, WeaponLevel::TwoWay);
    g.players[0].invulnerable = 0;
    g.players[0].passengers = 2;
    g.projectiles
        .push(Projectile::enemy(g.players[0].position, Vec2::ZERO));
    step(&mut g, Input::default());
    assert_eq!(g.players[0].lives, 5);
    assert_eq!(g.players[0].weapon, WeaponLevel::Grenade);
    assert_eq!(g.players[0].passengers, 0);
    for _ in 0..115 {
        step(&mut g, Input::default());
    }
    assert!(g.players[0].active());
    assert!(g.players[0].invulnerable > 0);
    assert!(assets().levels[0].clear_for_jeep(g.players[0].position, false));
}
#[test]
fn explicit_new_game_from_paused_session_starts_unpaused() {
    let mut g = Game::new();
    g.start_new_game(1, assets());
    step(
        &mut g,
        Input {
            pause: true,
            ..Default::default()
        },
    );
    g.start_new_game(2, assets());
    assert!(!g.paused, "new game inherited old pause state");
    let clock = g.tick;
    step(&mut g, Input::default());
    assert!(g.tick > clock);
}

#[test]
fn original_odd_distance_event_is_skipped_until_high_difficulty() {
    let mut isolated = Assets::load(&Assets::default_directory()).unwrap();
    // Isolate one authentic actor event so its difficulty rule cannot be hidden
    // by other actors with the same type or by the rendering cull boundary.
    let event = isolated.levels[0]
        .events
        .iter()
        .find(|e| {
            e.odd_difficulty_only
                && Behavior::from_family(&isolated.entity(e.source_type).family).is_some()
        })
        .unwrap()
        .clone();
    isolated.levels[0].events = vec![event.clone()];
    isolated.levels[0].commands.clear();
    for difficulty in [0, 2, 3] {
        let mut game = Game::new();
        game.start_new_game(1, &isolated);
        game.difficulty = difficulty;
        game.camera.y = isolated.levels[0].start_camera_y() - (event.distance - 240.);
        protect_and_center(&mut game);
        game.update(Input::default(), &isolated);
        let count = game
            .enemies
            .iter()
            .filter(|e| e.definition == event.source_type)
            .count();
        assert_eq!(
            count,
            usize::from(difficulty >= 3),
            "original event at {} on difficulty {difficulty}",
            event.distance
        );
    }
}
