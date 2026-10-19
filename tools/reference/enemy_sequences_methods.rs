impl Game {
    pub fn actor_call(&mut self, address: u16, slot: u8) -> Result<(), String> {
        self.bank = 6;
        self.memory[0x27] = 6;
        self.x = slot;
        self.memory[0x35] = slot;
        self.push_word(0xc28e);
        self.block = address;
        self.run_until_idle()
    }
}
