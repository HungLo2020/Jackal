mod audio;
mod game;
mod video;
use game::Game;
fn main() {
    let mut captures = vec![];
    for patch in 0..29u8 {
        let mut game = Game::new(false).unwrap();
        game.memory.fill(0);
        game.memory[0x27] = 6;
        game.memory[0x300..0x500].fill(255);
        game.memory[1] = 64;
        game.memory[2] = 64;
        game.memory[3] = patch;
        let entry = if (8..12).contains(&patch) {0xdf63} else if [14,26].contains(&patch) {0xdf67} else {0xdf6e};
        game.actor_call(entry, 0).unwrap();
        let cells: Vec<_> = game.memory[0x300..0x500].iter().enumerate().flat_map(|(byte,&packed)| {
            (0..4).filter_map(move |cell| {
                let value = (packed >> (6-cell*2)) & 3;
                (value != 3).then_some(serde_json::json!({"cell":byte*4+cell,"collision":value}))
            })
        }).collect();
        captures.push(serde_json::json!({"patch":patch,"cells":cells}));
    }
    println!("{}",serde_json::to_string(&captures).unwrap());
}
