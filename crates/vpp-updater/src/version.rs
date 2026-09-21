//! Comparing site versions, for rollback protection: the updater never
//! replaces an installed page with an older version (`ARCHITECTURE.md`,
//! invariant 3).

use std::cmp::Ordering;

/// Compares dotted version strings segment by segment: numerically where both
/// segments are numbers, as text otherwise. A missing segment sorts first, so
/// `1.2` is older than `1.2.1`. The same rules as the C++ updater.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (number(x), number(y)) {
                    (Some(nx), Some(ny)) => nx.cmp(&ny),
                    _ if x.is_empty() && !y.is_empty() => Ordering::Less,
                    _ if y.is_empty() && !x.is_empty() => Ordering::Greater,
                    _ => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// The segment as a number, if it is all digits. Too many digits compare as text.
fn number(segment: &str) -> Option<u64> {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    segment.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::{Equal, Greater, Less};

    #[test]
    fn numeric_segments_compare_as_numbers() {
        assert_eq!(compare_versions("1.10.0", "1.9.9"), Greater);
        assert_eq!(compare_versions("2.0", "10.0"), Less);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Equal);
        assert_eq!(compare_versions("01.2", "1.2"), Equal);
    }

    #[test]
    fn a_missing_segment_is_older() {
        assert_eq!(compare_versions("1.2", "1.2.1"), Less);
        assert_eq!(compare_versions("1.2.0", "1.2"), Greater);
    }

    #[test]
    fn text_segments_compare_as_text() {
        assert_eq!(compare_versions("1.0.beta", "1.0.alpha"), Greater);
        assert_eq!(compare_versions("1.0.rc1", "1.0.2"), Greater); // "rc1" > "2" as text
    }
}
