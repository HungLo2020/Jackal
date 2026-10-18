mod audio;
mod game;
mod video;
mod window;
use game::Game;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::{
    error::Error,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--headless") {
        let frames: usize = args
            .get(pos + 1)
            .ok_or("--headless needs frame count")?
            .parse()?;
        let mut game = Game::new(false)?;
        for f in 0..frames {
            let input = if f == 120 {
                8
            } else if f > 500 {
                0x11
            } else {
                0
            };
            game.update([input, 0])?;
        }
        let pixels = game.video.render();
        if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
            let path = args.get(pos + 1).ok_or("--screenshot needs PPM path")?;
            let mut bytes = b"P6\n256 240\n255\n".to_vec();
            for p in pixels {
                bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            std::fs::write(path, bytes)?;
        }
        println!(
            "Completed {} native frames; {} translated entries; state={:02x} bank={}",
            game.frame, game.transfers, game.memory[0x18], game.bank
        );
        return Ok(());
    }
    let mut game = Game::new(!args.iter().any(|a| a == "--mute"))?;
    let mut window = Window::new(
        "Jackal — native Rust | Enter: Start | Z/X: Fire | Esc: Exit",
        256,
        240,
        WindowOptions {
            scale: Scale::X2,
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window::configure(&window)?;
    window.set_target_fps(0);
    let mut paused = false;
    let mut next = Instant::now();
    let step = Duration::from_nanos(16_639_267);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            paused = !paused;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            game = Game::new(!args.iter().any(|a| a == "--mute"))?;
        }
        let layouts = [
            [
                Key::Z,
                Key::X,
                Key::Tab,
                Key::Enter,
                Key::Up,
                Key::Down,
                Key::Left,
                Key::Right,
            ],
            [
                Key::G,
                Key::H,
                Key::T,
                Key::Y,
                Key::W,
                Key::S,
                Key::A,
                Key::D,
            ],
        ];
        let mut buttons = [0; 2];
        for (p, keys) in layouts.iter().enumerate() {
            for (i, k) in keys.iter().enumerate() {
                if window.is_key_down(*k) {
                    buttons[p] |= 1 << i;
                }
            }
        }
        if !paused {
            game.update(buttons)?;
        }
        window.update_with_buffer(game.video.render(), 256, 240)?;
        next += step;
        if let Some(delay) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay);
        } else {
            next = Instant::now();
        }
    }
    Ok(())
}
