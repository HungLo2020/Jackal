use jackal_native::{
    assets::Assets,
    game::{Game, GameEvent, GameState},
    input::Input,
    renderer::Renderer,
};
use std::sync::OnceLock;
fn assets() -> &'static Assets {
    static A: OnceLock<Assets> = OnceLock::new();
    A.get_or_init(|| Assets::load(&Assets::default_directory()).unwrap())
}
#[test]
fn helicopter_is_present_in_real_game_flow_and_start_cannot_skip_it() {
    for players in [1, 2] {
        let mut game = Game::new();
        game.selected_players = players;
        game.update(
            Input {
                start: true,
                ..Default::default()
            },
            assets(),
        );
        assert_eq!(game.state, GameState::Briefing { ticks: 0 });
        assert!(game
            .drain_events()
            .iter()
            .any(|e| matches!(e, GameEvent::AudioCommand(0x17))));
        for tick in 1..=546 {
            game.update(
                Input {
                    start: tick % 2 == 0,
                    ..Default::default()
                },
                assets(),
            );
            assert_eq!(game.state, GameState::Briefing { ticks: tick });
        }
        let mut renderer = Renderer::new();
        let actual = renderer.render(&game, assets());
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/cinematics/intro_{players}p_tick_546.png"
        ));
        let expected = image::open(path).unwrap().to_rgb8();
        for (actual, p) in actual.iter().zip(expected.pixels()) {
            assert_eq!(
                *actual & 0xffffff,
                (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])
            );
        }
        let duration = if players == 1 { 632 } else { 648 };
        for _ in 547..duration {
            game.update(Input::default(), assets());
            assert!(matches!(game.state, GameState::Briefing { .. }));
        }
        game.update(Input::default(), assets());
        assert_eq!(game.state, GameState::Playing);
    }
}
#[test]
fn continue_waits_for_a_choice_instead_of_timing_out_or_exiting_on_select() {
    let mut game = Game::new();
    game.state = GameState::Continue { ticks: 0 };
    for _ in 0..1800 {
        game.update(Input::default(), assets());
    }
    assert!(matches!(game.state, GameState::Continue { .. }));
    game.update(
        Input {
            select: true,
            ..Default::default()
        },
        assets(),
    );
    assert!(!game.continue_selected);
    assert!(matches!(game.state, GameState::Continue { .. }));
    game.update(
        Input {
            start: true,
            ..Default::default()
        },
        assets(),
    );
    assert_eq!(game.state, GameState::Title);
}
#[test]
fn rescued_prisoners_are_tallied_at_original_cadence_for_2000_points_each() {
    let mut game = Game::new();
    game.start_new_game(2, assets());
    game.players[0].delivered = 2;
    game.players[1].delivered = 1;
    game.players[0].passengers = 4;
    game.state = GameState::StageClear { ticks: 211 };
    game.tick = 7;
    game.update(Input::default(), assets());
    assert_eq!(game.players[0].score, 2000);
    assert_eq!(game.players[1].score, 2000);
    assert_eq!(game.players[0].delivered, 1);
    assert_eq!(game.players[1].delivered, 0);
    for _ in 0..7 {
        game.update(Input::default(), assets());
    }
    assert_eq!(game.players[0].score, 2000);
    game.update(Input::default(), assets());
    assert_eq!(game.players[0].score, 4000);
    assert_eq!(
        game.players[0].passengers, 4,
        "carried prisoners are not silently tallied"
    );
}

#[test]
fn texture_stream_transitions_preserve_intermediate_and_final_atlases() {
    let assets = Assets::load(&Assets::default_directory()).unwrap();
    for (stage, phases) in assets.texture_phases.iter().enumerate() {
        for phase in phases {
            let (tiles, sprites) = assets.textures(stage, phase.distance, phase.tick_offset);
            assert!(std::ptr::eq(tiles, &phase.tiles));
            assert!(std::ptr::eq(sprites, &phase.sprites));
            if phase.tick_offset > 0 {
                let (before, _) = assets.textures(stage, phase.distance, phase.tick_offset - 1);
                assert!(
                    !std::ptr::eq(before, &phase.tiles),
                    "stage{stage} transition must not appear early"
                );
            }
        }
    }
}
