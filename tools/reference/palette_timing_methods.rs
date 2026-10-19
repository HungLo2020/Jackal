impl Game {
    pub fn scheduler_call(&mut self) -> Result<(), String> {
        self.bank = 6;
        self.memory[0x27] = 6;
        self.push_word(0xc28e);
        self.block = 0xddb0;
        self.run_until_idle()
    }
}
