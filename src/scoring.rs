/// Original E648 score lookup, also used by actor spawn bonus indexing.
pub const SCORE_VALUES: [u32; 16] = [
    0, 50, 100, 200, 300, 500, 800, 1000, 1500, 2000, 3000, 5000, 8000, 9000, 9990, 10,
];
pub fn cooperative_score(score: u32) -> u32 {
    SCORE_VALUES
        .iter()
        .position(|&value| value == score)
        .filter(|&index| index > 0 && index < 15)
        .map_or(score, |index| SCORE_VALUES[index + 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooperative_spawns_advance_nonzero_original_score_index() {
        assert_eq!(cooperative_score(0), 0);
        assert_eq!(cooperative_score(50), 100);
        assert_eq!(cooperative_score(500), 800);
        assert_eq!(cooperative_score(9990), 10);
        assert_eq!(SCORE_VALUES[15], 10, "paratrooper infantry override");
    }
}
