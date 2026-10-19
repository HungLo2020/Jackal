mod window;
use jackal_native::{
    assets::Assets,
    game::{Game, GameEvent},
    input::{Input, PlayerInput},
    renderer::{Renderer, HEIGHT, WIDTH},
    sound::Sound,
    TICKS_PER_SECOND,
};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::{
    error::Error,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Options {
    headless: Option<u64>,
    screenshot: Option<PathBuf>,
    stage: Option<usize>,
    mute: bool,
    assets: Option<PathBuf>,
}
impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut result = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--headless" => {
                    result.headless = Some(
                        args.next()
                            .ok_or("--headless needs a frame count")?
                            .parse()?,
                    )
                }
                "--stage" => {
                    let stage: usize = args.next().ok_or("--stage needs 1..6")?.parse()?;
                    if !(1..=6).contains(&stage) {
                        return Err("stage must be between 1 and 6".into());
                    }
                    result.stage = Some(stage - 1);
                }
                "--screenshot" => {
                    result.screenshot =
                        Some(args.next().ok_or("--screenshot needs a PNG path")?.into())
                }
                "--assets" => {
                    result.assets = Some(args.next().ok_or("--assets needs a directory")?.into())
                }
                "--mute" => result.mute = true,
                "--help" | "-h" => {
                    println!("Jackal native Rust\n  cargo run [-- --mute] [--stage 1..6]\n  --headless FRAMES --screenshot FILE.png\n  --assets DIRECTORY\nArrows/Z(grenade)/X(gun)/Enter/Tab; player 2 WASD/G/H/Y/T.\nP: pause, R: restart, Escape: exit.");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown option: {arg}").into()),
            }
        }
        if result.screenshot.is_some() && result.headless.is_none() {
            return Err("--screenshot requires --headless".into());
        }
        Ok(result)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let assets = Assets::load(&options.assets.unwrap_or_else(Assets::default_directory))?;
    let mut game = Game::new();
    let mut renderer = Renderer::new();
    if let Some(stage) = options.stage {
        game.start_new_game(1, &assets);
        game.start_stage(stage, &assets);
    }
    if let Some(frames) = options.headless {
        for tick in 0..frames {
            let input = Input {
                start: tick == 60,
                players: [
                    PlayerInput {
                        up: tick > 240,
                        gun: tick > 240,
                        grenade: tick > 240 && tick.is_multiple_of(48),
                        ..PlayerInput::default()
                    },
                    PlayerInput::default(),
                ],
                ..Input::default()
            };
            game.update(input, &assets);
            game.drain_events();
        }
        let pixels = renderer.render(&game, &assets);
        if let Some(path) = options.screenshot {
            let bytes: Vec<u8> = pixels
                .iter()
                .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
                .collect();
            image::save_buffer_with_format(
                path,
                &bytes,
                WIDTH as u32,
                HEIGHT as u32,
                image::ColorType::Rgb8,
                image::ImageFormat::Png,
            )?;
        }
        println!(
            "Completed {frames} native ticks; stage {}; state {:?}",
            game.stage + 1,
            game.state
        );
        return Ok(());
    }
    let mut sound = Sound::new(!options.mute);
    let mut window = Window::new(
        "Jackal — native Rust | Z: Grenade | X: Gun | Enter: Start",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window::configure(&window)?;
    window.set_target_fps(120);
    let step = Duration::from_secs_f64(1. / TICKS_PER_SECOND);
    let mut previous = Instant::now();
    let mut accumulator = Duration::ZERO;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            game = Game::new();
            sound.pause(false);
        }
        let now = Instant::now();
        accumulator += (now - previous).min(step * 5);
        previous = now;
        let mut input = Input {
            start: window.is_key_down(Key::Enter) || window.is_key_down(Key::Y),
            select: window.is_key_down(Key::Tab) || window.is_key_down(Key::T),
            pause: window.is_key_down(Key::P),
            ..Input::default()
        };
        for (i, keys) in [
            [Key::Up, Key::Down, Key::Left, Key::Right, Key::Z, Key::X],
            [Key::W, Key::S, Key::A, Key::D, Key::G, Key::H],
        ]
        .iter()
        .enumerate()
        {
            input.players[i] = PlayerInput {
                up: window.is_key_down(keys[0]),
                down: window.is_key_down(keys[1]),
                left: window.is_key_down(keys[2]),
                right: window.is_key_down(keys[3]),
                grenade: window.is_key_down(keys[4]),
                gun: window.is_key_down(keys[5]),
            };
        }
        while accumulator >= step {
            game.update(input, &assets);
            accumulator -= step;
        }
        for event in game.drain_events() {
            match event {
                GameEvent::Music(track) => sound.music(track),
                GameEvent::AudioCommand(command) => sound.command(command),
                GameEvent::Effect(effect) => sound.effect(effect),
                GameEvent::Pause(paused) => sound.pause(paused),
            }
        }
        window.update_with_buffer(renderer.render(&game, &assets), WIDTH, HEIGHT)?;
    }
    Ok(())
}
