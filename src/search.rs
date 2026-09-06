use std::ops::Range;

/// Byte ranges of the non-overlapping occurrences of `query` in `text`, ignoring case.
pub fn match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let query: Vec<char> = query.chars().map(fold_case).collect();
    if query.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut ranges = Vec::new();
    let mut start = 0;
    while start + query.len() <= chars.len() {
        let window = &chars[start..start + query.len()];
        if window.iter().zip(&query).all(|(&(_, c), &expected)| fold_case(c) == expected) {
            let (first, _) = window[0];
            let (last, last_char) = window[window.len() - 1];
            ranges.push(first..last + last_char.len_utf8());
            start += query.len();
        } else {
            start += 1;
        }
    }
    ranges
}

// One character in, one character out: the query and the text stay aligned character by character.
fn fold_case(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// The item a search lands on from `current`: the first match going `forward` (or back), wrapping around.
/// `current` itself is tried last, so a lone match is found again.
pub fn next_match<S: AsRef<str>>(items: &[S], query: &str, current: usize, forward: bool) -> Option<usize> {
    let len = items.len();
    if len == 0 {
        return None;
    }
    let current = current % len;
    (1..=len)
        .map(|step| if forward { (current + step) % len } else { (current + len - step) % len })
        .find(|&index| !match_ranges(items[index].as_ref(), query).is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_ranges_ignores_case_and_lists_every_occurrence() {
        assert_eq!(match_ranges("A Promo promo", "PROMO"), vec![2..7, 8..13]);
    }

    #[test]
    fn match_ranges_is_empty_without_a_query_or_a_match() {
        assert!(match_ranges("anything", "").is_empty());
        assert!(match_ranges("anything", "zzz").is_empty());
    }

    #[test]
    fn match_ranges_returns_byte_offsets_on_multibyte_text() {
        // "Été" is 5 bytes: É (2) t (1) é (2).
        assert_eq!(match_ranges("Été", "été"), vec![0..5]);
        assert_eq!(match_ranges("a Été", "té"), vec![4..7]);
    }

    #[test]
    fn next_match_goes_forward_and_wraps_to_the_first_match() {
        let items = ["a", "b a", "c"];
        assert_eq!(next_match(&items, "a", 0, true), Some(1));
        assert_eq!(next_match(&items, "a", 1, true), Some(0));
    }

    #[test]
    fn next_match_goes_backward_and_wraps_to_the_last_match() {
        let items = ["a", "b a", "c"];
        assert_eq!(next_match(&items, "a", 0, false), Some(1));
        assert_eq!(next_match(&items, "a", 1, false), Some(0));
    }

    #[test]
    fn next_match_finds_the_only_match_again_and_none_when_absent() {
        let items = ["x", "a"];
        assert_eq!(next_match(&items, "a", 1, true), Some(1));
        assert_eq!(next_match(&items, "zzz", 0, true), None);
        assert_eq!(next_match::<&str>(&[], "a", 0, true), None);
    }
}
