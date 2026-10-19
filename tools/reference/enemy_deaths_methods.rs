impl Game {
 pub fn death_call(&mut self,address:u16)->Result<Vec<u8>,String>{
  self.bank=6;self.memory[0x27]=6;self.x=0;self.memory[0x35]=0;self.push_word(0xc28e);self.block=address;
  let mut commands=Vec::new();
  for _ in 0..2000000 {
   if self.block==0xc28f{return Ok(commands);}
   if self.block==0xc36b {commands.push(self.a);}
   self.block=translated::dispatch(self,self.block)?;
  }
  Err("Death capture did not return".into())
 }
}
