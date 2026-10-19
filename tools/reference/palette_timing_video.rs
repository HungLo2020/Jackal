impl Video {
    pub fn palette_evidence(&self) -> Vec<u8> {
        self.memory[0x3f00..0x3f10].to_vec()
    }
}
