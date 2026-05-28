fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_two_positives() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn add_two_negatives() {
        assert_eq!(add(-2, -3), -5);
    }

    #[test]
    fn add_positive_and_negative() {
        assert_eq!(add(10, -3), 7);
    }

    #[test]
    fn add_positive_and_negative_result_zero() {
        assert_eq!(add(5, -5), 0);
    }

    #[test]
    fn add_with_zero() {
        assert_eq!(add(0, 42), 42);
        assert_eq!(add(42, 0), 42);
    }

    #[test]
    fn add_two_zeros() {
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn add_i32_max_and_one_overflows() {
        add(i32::MAX, 1);
    }

    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn add_i32_min_and_negative_one_overflows() {
        add(i32::MIN, -1);
    }

    #[test]
    fn add_near_bounds_no_overflow() {
        assert_eq!(add(i32::MAX, 0), i32::MAX);
        assert_eq!(add(i32::MAX, -1), i32::MAX - 1);
        assert_eq!(add(i32::MIN, 0), i32::MIN);
        assert_eq!(add(i32::MIN, 1), i32::MIN + 1);
    }
}
