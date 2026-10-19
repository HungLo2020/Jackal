use jackal_native::{math::Vec2, rescue::Prisoner, terrain::CollisionField};
struct Ground;
impl CollisionField for Ground {
    fn terrain_at(&self, _: Vec2) -> u8 {
        0
    }
}
#[test]
fn prisoner_states_match_original_execution() {
    let traces: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/prisoners.json")).unwrap();
    for trace in traces.as_array().unwrap() {
        let flags = trace["flags"].as_u64().unwrap() as u8;
        let mut p = Prisoner::released(
            Vec2::new(120., 120.),
            flags,
            if flags == 1 { 8 } else { 0 },
            0,
        );
        for row in trace["samples"].as_array().unwrap() {
            let tick = row["tick"].as_u64().unwrap();
            p.tick(tick, false, &Ground);
            assert_eq!(
                (p.position.x, p.position.y, p.sprite, p.palette, p.flip_x),
                (
                    row["x"].as_f64().unwrap() as f32,
                    row["y"].as_f64().unwrap() as f32,
                    row["sprite"].as_u64().unwrap() as u16,
                    row["palette"].as_u64().unwrap() as u8,
                    row["flip_x"].as_bool().unwrap()
                ),
                "flags{flags} tick{tick}"
            );
        }
    }
}
#[test]
fn prison_variants_preserve_release_counts_smoke_and_predecessor_wait() {
    use jackal_native::rescue::PrisonRelease;
    for (source, count, first) in [
        (19, 2, 90),
        (20, 2, 90),
        (28, 4, 90),
        (29, 4, 90),
        (21, 1, 25),
        (25, 0, 90),
    ] {
        let mut release = PrisonRelease::new(Vec2::ZERO, source);
        for age in 0..24 {
            assert_eq!(release.sprite(), Some((25 + age / 8, 3)));
            assert!(!release.tick(true));
        }
        let mut total = 0;
        for age in 25..150 {
            let spawned = release.tick(true);
            if spawned {
                total += 1;
                assert!(age >= first);
            }
        }
        assert_eq!(total, count, "source{source}");
    }
    let mut p = Prisoner::released(Vec2::new(120., 120.), 0, 0, 2);
    for tick in 0..200 {
        p.tick(tick, true, &Ground);
        assert!(!p.collectible());
        assert_eq!(p.position, Vec2::new(120., 120.));
    }
    p.tick(200, false, &Ground);
    for tick in 201..273 {
        p.tick(tick, false, &Ground);
    }
    assert!(p.collectible());
    assert_eq!(p.position, Vec2::new(156., 120.));
}
#[test]
fn unloaded_prisoner_walks_96_ticks_before_arrival() {
    use jackal_native::rescue::Delivery;
    let mut d = Delivery {
        actor_handle: None,
        position: Vec2::new(120., 120.),
        owner: 0,
        remaining: 96,
        last: true,
        palette: 0,
        left: true,
    };
    for _ in 0..95 {
        assert!(!d.tick());
    }
    assert!(d.tick());
    assert_eq!(d.position, Vec2::new(72., 120.));
    assert!(!d.tick());
}
#[test]
fn destroyed_jeep_releases_only_surviving_passengers_one_per_tick() {
    use jackal_native::rescue::PassengerDrop;
    for passengers in 0..=8 {
        let mut drop = PassengerDrop::new(Vec2::new(20., 30.), passengers);
        let mut total = 0;
        for tick in 0..8 {
            if let Some(p) = drop.next(tick, tick) {
                total += 1;
                assert_eq!(p.position, Vec2::new(20., 30.));
                assert!(!p.collectible());
            }
        }
        assert_eq!(total, passengers.saturating_sub(1).min(4));
        assert!(drop.finished());
    }
}
