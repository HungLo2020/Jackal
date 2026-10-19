use jackal_native::{assets::Assets, input::PlayerInput, math::Vec2, player::Player};
#[test]
fn death_frames_and_life_decrement_follow_original_trace() {
    let assets = Assets::load(&Assets::default_directory()).unwrap();
    let trace: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/player_death.json")).unwrap();
    let mut player = Player::new(Vec2::new(80., 128.), true);
    player.invulnerable = 0;
    player.passengers = 3;
    assert!(player.hit());
    for row in trace["samples"].as_array().unwrap() {
        let tick = row[0].as_u64().unwrap();
        if tick > 0 {
            player.tick(PlayerInput::default(), &assets.levels[0], 0, tick);
        }
        assert_eq!(
            player.lives,
            u8::try_from(row[4].as_u64().unwrap()).unwrap(),
            "tick{tick}"
        );
        assert_eq!(player.passengers, 0);
        let frame = if player.respawn_ticks > 0 {
            assets.animations["jeep_death"]
                .frames
                .get((tick / 6) as usize)
                .copied()
                .unwrap_or(0)
        } else {
            1
        };
        assert_eq!(frame, row[2].as_u64().unwrap() as u16, "tick{tick}");
    }
}
#[test]
fn pickup_and_unloading_do_not_award_invented_points() {
    let mut player = Player::new(Vec2::ZERO, true);
    assert!(player.rescue(false));
    assert!(player.deliver_one());
    assert_eq!(
        player.score, 0,
        "original E581 and E997 award no points; stage tally awards2000 per delivered prisoner"
    );
}

#[test]
fn river_current_motion_matches_original_for_all_directions_and_blocked_ground() {
    use jackal_native::terrain::CollisionField;
    struct Ground(u8);
    impl CollisionField for Ground {
        fn terrain_at(&self, _: Vec2) -> u8 {
            self.0
        }
    }
    let traces: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/river_current.json")).unwrap();
    for trace in traces.as_array().unwrap() {
        let bits = trace["input"].as_u64().unwrap();
        let mut p = Player::new(Vec2::new(128., 128.), true);
        p.river_current = true;
        for row in trace["samples"].as_array().unwrap() {
            let tick = row["tick"].as_u64().unwrap();
            p.tick(
                PlayerInput {
                    right: bits & 1 != 0,
                    left: bits & 2 != 0,
                    down: bits & 4 != 0,
                    up: bits & 8 != 0,
                    ..PlayerInput::default()
                },
                &Ground(trace["terrain"].as_u64().unwrap() as u8),
                5,
                tick,
            );
            assert_eq!(
                p.position,
                Vec2::new(
                    row["x"].as_f64().unwrap() as f32,
                    row["y"].as_f64().unwrap() as f32
                ),
                "input{bits} tick{tick}"
            );
        }
    }
}

#[test]
fn land_mine_reveal_and_palette_match_original_reference() {
    use jackal_native::mine::LandMine;
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mines.json")).unwrap();
    let mut mine = LandMine::new(Vec2::new(120., 120.));
    for row in rows.as_array().unwrap() {
        mine.tick([
            Some(Vec2::new(row["player_x"].as_f64().unwrap() as f32, 120.)),
            None,
        ]);
        assert_eq!(
            (mine.sprite, mine.palette),
            (
                row["sprite"].as_u64().unwrap() as u16,
                row["palette"].as_u64().unwrap() as u8
            ),
            "tick{}",
            row["tick"]
        );
    }
}
