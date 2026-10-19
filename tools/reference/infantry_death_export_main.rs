// Offline reference driver: use historical game/audio/video and appended video_export_methods.rs.
// Emits raw capture JSON to stdout and screenshot to argv[1].
mod audio;mod game;mod video;
use game::Game;
fn main(){
 let output=std::env::args().nth(1).expect("output directory");
 let mut g=Game::new(false).unwrap();for f in 0..900 {g.update([if f==120{8}else{0},0]).unwrap();}
 g.memory[0x30]=0;g.memory[0x18]=5;g.memory[0x2c]=1;g.memory[0x34]=1;for _ in 0..90{g.update([0;2]).unwrap();}
 let mut captures=Vec::new();
 for typ in 1..=4 {
  for i in 0..16 {g.memory[0x720+i]=0;}
  let i=0;g.memory[0x720+i]=typ;g.memory[0x520+i]=if typ==1 || typ==4{7}else{3};
  g.memory[0x540+i]=0xff;g.memory[0x500+i]=18;
  g.memory[0x5a0+i]=g.memory[0x5b0];g.memory[0x560+i]=g.memory[0x570]-32;
  g.memory[0x6f0+i]=0;g.memory[0x710+i]=80;
  g.memory[0x700+i]=0;g.memory[0x580+i]=0;
  let mut samples=Vec::new();
  for tick in 0..35 {
   g.update([0;2]).unwrap();
   if typ==1 && tick==3 {g.video.screenshot(&format!("{output}/infantry_death.png"));}
   samples.push(serde_json::json!({"tick":tick,"type":g.memory[0x720],"state":g.memory[0x520],"sprite":g.memory[0x500],"attribute":g.memory[0x540],"timer":g.memory[0x6d0],"x":g.memory[0x5a0],"y":g.memory[0x560]}));
  }
  captures.push(serde_json::json!({"source_type":typ,"samples":samples}));
 }
 println!("{}",serde_json::to_string_pretty(&captures).unwrap());
}
