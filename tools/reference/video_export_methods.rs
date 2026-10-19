
impl Video {
 pub fn export(&self,dir:&str,stage:usize){
  use image::{Rgba,RgbaImage};
  let mut bg=RgbaImage::new(128,512);
  let mut sprites=RgbaImage::new(128,1024);
  for pal in 0..4 {for tile in 0..256 {
   for y in 0..8 {for x in 0..8 {
    let v=self.tile_pixel(((self.ctrl as usize&16)<<8)+tile*16,x,y);
    let rgb=self.color(if v==0 {0x3f00}else{0x3f00+pal*4+v},0);
    bg.put_pixel(((tile%16)*8+x) as u32,((pal*16+tile/16)*8+y)as u32,Rgba([(rgb>>16)as u8,(rgb>>8)as u8,rgb as u8,255]));
   }}
   for y in 0..16 {for x in 0..8 {
    let addr=(tile&1)*0x1000+(tile&!1)*16+(y/8)*16;
    let v=self.tile_pixel(addr,x,y%8);
    let rgb=self.color(0x3f10+pal*4+v,0);
    sprites.put_pixel(((tile%16)*8+x)as u32,((pal*16+tile/16)*16+y)as u32,Rgba([(rgb>>16)as u8,(rgb>>8)as u8,rgb as u8,if v==0{0}else{255}]));
   }}
  }}
  let palette:Vec<String>=(0..32).map(|i|format!("#{:06x}",self.color(0x3f00+i,0))).collect();
  std::fs::write(format!("{dir}/stage_{stage:02}_palette.json"),serde_json::to_string_pretty(&palette).unwrap()+"\n").unwrap();
  bg.save(format!("{dir}/stage_{stage:02}_tiles.png")).unwrap();
  sprites.save(format!("{dir}/stage_{stage:02}_sprites.png")).unwrap();
 }
 pub fn screenshot(&mut self,path:&str){
  let px=self.render();let mut img=image::RgbImage::new(256,240);
  for (i,c)in px.iter().enumerate(){img.put_pixel((i%256)as u32,(i/256)as u32,image::Rgb([(c>>16)as u8,(c>>8)as u8,*c as u8]));}img.save(path).unwrap();
 }
}

impl Video { pub fn export_snapshot(&self)->[u8;0x4000]{*self.memory} pub fn restore_export_snapshot(&mut self,memory:[u8;0x4000]){*self.memory=memory;} }
