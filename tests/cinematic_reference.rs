//! Pixel comparisons against frames rendered by the historical reference runtime.
use jackal_native::{cinematic::Cinematics, renderer::Canvas};
use std::{path::PathBuf, sync::OnceLock};

fn cinematics() -> &'static Cinematics {
    static ASSETS: OnceLock<Cinematics> = OnceLock::new();
    ASSETS.get_or_init(|| {
        Cinematics::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap()
    })
}
fn compare(clip: &str, tick: u32) {
    let mut canvas = Canvas::new(256, 240);
    cinematics().get(clip).unwrap().draw(&mut canvas, tick);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/cinematics/{clip}_tick_{tick}.png"));
    let reference = image::open(path).unwrap().into_rgb8();
    for (index, pixel) in reference.pixels().enumerate() {
        let expected = ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32;
        assert_eq!(
            canvas.pixels()[index] & 0xffffff,
            expected,
            "{clip} tick {tick}, pixel {},{}",
            index % 256,
            index / 256
        );
    }
}
#[test]
fn mission_map_helicopter_arrival_jeep_drop_and_departure_match_original() {
    for players in 1..=2 {
        for tick in [82, 300, 546, 631] {
            compare(&format!("intro_{players}p"), tick);
        }
    }
}
#[test]
fn ending_scenes_match_original() {
    for tick in [1, 600, 1200, 1800, 3000, 4800, 5161] {
        compare("ending", tick);
    }
}
#[test]
fn intro_duration_preserves_second_player_drop() {
    assert_eq!(cinematics().get("intro_1p").unwrap().duration_ticks(), 632);
    assert_eq!(cinematics().get("intro_2p").unwrap().duration_ticks(), 648);
    assert_eq!(cinematics().get("ending").unwrap().duration_ticks(), 5162);
}
#[test]
fn clip_end_holds_its_final_frame() {
    let clip = cinematics().get("intro_1p").unwrap();
    let mut final_frame = Canvas::new(256, 240);
    let mut past_end = Canvas::new(256, 240);
    clip.draw(&mut final_frame, clip.duration_ticks() - 1);
    clip.draw(&mut past_end, u32::MAX);
    assert_eq!(final_frame.pixels(), past_end.pixels());
}
#[test]
fn sound_cues_follow_the_recovered_timeline() {
    for name in ["intro_1p", "intro_2p"] {
        let clip = cinematics().get(name).unwrap();
        assert_eq!(clip.sound_commands_at(0).collect::<Vec<_>>(), [0x17]);
        assert_eq!(clip.sound_commands_at(82).count(), 0);
        assert_eq!(clip.sound_commands_at(83).collect::<Vec<_>>(), [0x34]);
    }
    let ending = cinematics().get("ending").unwrap();
    assert_eq!(
        ending.sound_commands_at(1025).collect::<Vec<_>>(),
        [0x59, 0x28]
    );
    assert_eq!(ending.sound_commands_at(1762).collect::<Vec<_>>(), [0x54]);
}
#[test]
fn original_sprite_hud_matches_reference_pixels() {
    use jackal_native::{hud::Hud, math::Vec2, player::Player};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let hud = Hud::load(&root.join("assets")).unwrap();
    for count in 1..=2 {
        let players = [
            Player::new(Vec2::new(0.0, 0.0), true),
            Player::new(Vec2::new(0.0, 0.0), count == 2),
        ];
        let mut canvas = Canvas::new(256, 240);
        hud.draw(&mut canvas, &players, 900);
        let image = image::open(root.join(format!("tests/fixtures/cinematics/hud_{count}p.png")))
            .unwrap()
            .into_rgba8();
        for (i, pixel) in image.pixels().enumerate() {
            let expected = if pixel[3] == 0 {
                0
            } else {
                ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32
            };
            assert_eq!(
                canvas.pixels()[192 * 256 + i] & 0xffffff,
                expected,
                "{count}p HUD pixel {},{}",
                i % 256,
                i / 256 + 192
            );
        }
    }
}
