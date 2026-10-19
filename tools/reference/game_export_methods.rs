impl Game {
    pub fn sound_call(&mut self, address: u16, command: u8) -> Result<(), String> {
        self.bank = 0;
        self.a = command;
        self.push_word(0xc28e);
        self.block = address;
        self.run_until_idle()
    }
}
