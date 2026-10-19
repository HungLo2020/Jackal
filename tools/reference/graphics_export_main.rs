mod audio;mod game;mod video;
use game::Game;
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let root=&args[1];
 std::fs::create_dir_all(format!("{root}/assets/textures")).unwrap();
 std::fs::create_dir_all(format!("{root}/tests/fixtures")).unwrap();
 let mut title=Game::new(false).unwrap();for _ in 0..100 {title.update([0;2]).unwrap();}
 title.video.screenshot(&format!("{root}/assets/textures/title.png"));
 for stage in 0..6 {
  let mut g=Game::new(false).unwrap();for f in 0..900 {g.update([if f==120{8}else{0},0]).unwrap();}
  g.memory[0x30]=stage;g.memory[0x18]=5;g.memory[0x2c]=1;g.memory[0x34]=1;
  for _ in 0..90{g.update([0;2]).unwrap();}
  g.video.export(&format!("{root}/assets/textures"),stage as usize+1);
  g.video.screenshot(&format!("{root}/tests/fixtures/stage_{:02}_start.png",stage+1));
  println!("stage{} x={} y={} scrollx={} scrolly={} page={} offset={} sprite={}",stage,g.memory[0x5b0],g.memory[0x570],g.memory[0xfd],g.memory[0xfc],g.memory[0x43],g.memory[0x44],g.memory[0x510]);
  if stage==0{
   let mut samples=Vec::new();
   for f in 0..80 {let input=if f<20{128}else if f<40{64}else if f<60{16}else{32};g.update([input,0]).unwrap();samples.push(serde_json::json!({"frame":f,"input":input,"x":g.memory[0x5b0],"y":g.memory[0x570],"camera_x":g.memory[0xfd],"camera_y":g.memory[0xfc],"frame_id":g.memory[0x510],"direction":g.memory[0x62],"flip":g.memory[0x7a]}));}
   std::fs::write(format!("{root}/tests/fixtures/player_motion.json"),serde_json::to_string_pretty(&samples).unwrap()).unwrap();
  }
 }
}
