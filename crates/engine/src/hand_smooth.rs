//! Arena-style BO1 hand smoothing: two shuffle samples, keep closer land count.

/// Whether sample A should be kept over sample B (ties → A).
pub(crate) fn sample_a_wins(lands_a: u32, lands_b: u32, expected: f64) -> bool {
    let dist_a = (lands_a as f64 - expected).abs();
    let dist_b = (lands_b as f64 - expected).abs();
    dist_a <= dist_b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closer_sample_wins() {
        // expected 3.0: A has 3, B has 5 → A
        assert!(sample_a_wins(3, 5, 3.0));
        // expected 3.0: A has 1, B has 3 → B
        assert!(!sample_a_wins(1, 3, 3.0));
    }

    #[test]
    fn equal_distance_prefers_sample_a() {
        // |2-3| == |4-3|
        assert!(sample_a_wins(2, 4, 3.0));
    }
}
