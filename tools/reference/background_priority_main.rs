mod audio;
mod game;
mod video;
use game::Game;
fn main() {
    let mut g = Game::new(false).unwrap();
    for f in 0..900 {
        g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
    }
    g.memory[0x30] = 3;
    g.memory[0x18] = 5;
    g.memory[0x2c] = 1;
    g.memory[0x34] = 1;
    for _ in 0..90 {
        g.update([0; 2]).unwrap();
    }
    assert!(g.video.verify_background_mask());
    println!("stage4 initial mask exact");
    let mut tested = 0;
    for _ in 0..5000 {
        g.memory[0x52] = 255;
        g.memory[0x3c] = 1;
        g.update([0; 2]).unwrap();
        let distance = u32::from(g.memory[0x43]) * 240 + u32::from(g.memory[0x44]);
        if distance >= 2707 {
            assert!(g.video.verify_background_mask(), "distance{distance}");
            tested += 1;
            if tested == 40 {
                break;
            }
        }
    }
    println!("stage4 phase {tested} snapshots exact");
    assert_eq!(tested, 40);
}
