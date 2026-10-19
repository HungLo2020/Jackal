impl Audio {
    pub fn render(&mut self) -> Vec<i16> {
        self.sample_remainder += 44100.0 / 60.0988;
        let n = self.sample_remainder as usize;
        self.sample_remainder -= n as f64;
        (0..n)
            .map(|_| (self.synth.sample().clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect()
    }
}
