//! Version numbers as lists of numbers, for sorting and comparing. Two readings, because
//! versions come in two kinds:
//! - `release`: a release's own version ("v21.0.5+11", "8.4.0-RC1", "33.3.1-jre"): the
//!   numbers up to the first part that isn't one, so "-RC1" never outranks a release (a
//!   "+11" build number does count).
//! - `numbers`: every run of digits ("OTP-27.1.2", "mysql-8.4.3-winx64"), for folder names
//!   and catalogs whose versions carry words around the numbers.

/// The leading numeric parts: "v21.0.5+11" → [21, 0, 5, 11], "8.4.0-RC1" → [8, 4, 0].
pub fn release(v: &str) -> Vec<u64> {
    v.trim_start_matches(|c: char| !c.is_ascii_digit()).split(['.', '+', '-', '_']).map_while(|p| p.parse().ok()).collect()
}

/// Every number in it, in order: "OTP-27.1.2" → [27, 1, 2], "4.0.0-rc-4" → [4, 0, 0, 4].
pub fn numbers(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_kinds() {
        assert_eq!(release("v21.0.5+11"), [21, 0, 5, 11], "a build number counts");
        assert_eq!(release("8.4.0-RC1"), [8, 4, 0]);
        assert_eq!(release("33.3.1-jre"), [33, 3, 1]);
        assert_eq!(release("1.0_2"), [1, 0, 2]);
        assert!(release("v1.10.0") > release("1.9.3"));
        assert_eq!(numbers("OTP-27.1.2"), [27, 1, 2]);
        assert_eq!(numbers("4.0.0-rc-4"), [4, 0, 0, 4]);
        assert!(release("").is_empty() && numbers("latest").is_empty());
    }
}
