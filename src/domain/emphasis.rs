//! Word-level emphasis: in a modified line, the words that actually changed.

use std::ops::Range;

use super::diff::{DiffRow, RowKind};

// Past this many tokens per side, the LCS table is not worth building for one line.
const MAX_EMPHASIS_TOKENS: usize = 300;
// Beyond this share of the longer line, both lines are rewrites: the full-line background already says it.
const MAX_CHANGED_PERCENT: usize = 60;

// A run of removed rows followed by a run of added rows is a modification block: each removed row is paired
// with the added row at the same position, and the extra rows of the longer run stay without emphasis.
pub(super) fn emphasize_modifications(rows: &mut [DiffRow]) {
    let mut index = 0;
    while index < rows.len() {
        let removed_end = run_end(rows, index, RowKind::Removed);
        if removed_end == index {
            index += 1;
            continue;
        }
        let added_end = run_end(rows, removed_end, RowKind::Added);
        for (removed, added) in (index..removed_end).zip(removed_end..added_end) {
            let (old_emphasis, new_emphasis) = word_emphasis(&rows[removed].text, &rows[added].text);
            rows[removed].emphasis = old_emphasis;
            rows[added].emphasis = new_emphasis;
        }
        index = added_end;
    }
}

fn run_end(rows: &[DiffRow], start: usize, kind: RowKind) -> usize {
    start + rows[start..].iter().take_while(|row| row.kind == kind).count()
}

// The changed ranges of an old and a new line, or none on both sides when the pair is too different to highlight.
fn word_emphasis(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let old_tokens = tokens(old);
    let new_tokens = tokens(new);
    if old_tokens.len() > MAX_EMPHASIS_TOKENS || new_tokens.len() > MAX_EMPHASIS_TOKENS {
        return (Vec::new(), Vec::new());
    }
    let old_words: Vec<&str> = old_tokens.iter().map(|token| &old[token.clone()]).collect();
    let new_words: Vec<&str> = new_tokens.iter().map(|token| &new[token.clone()]).collect();
    let (old_common, new_common) = common_tokens(&old_words, &new_words);
    let old_changed = changed_ranges(&old_tokens, &old_common);
    let new_changed = changed_ranges(&new_tokens, &new_common);

    let changed = visible_chars(old, &old_changed).max(visible_chars(new, &new_changed));
    let longer = non_whitespace_chars(old).max(non_whitespace_chars(new));
    if changed * 100 > longer * MAX_CHANGED_PERCENT {
        return (Vec::new(), Vec::new());
    }
    (old_changed, new_changed)
}

// Words and numbers group together, whitespace groups together, every other character stands alone.
fn tokens(text: &str) -> Vec<Range<usize>> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut tokens: Vec<Range<usize>> = Vec::new();
    let mut previous: Option<char> = None;
    for (start, c) in text.char_indices() {
        let end = start + c.len_utf8();
        let joins_previous = previous.is_some_and(|previous| (is_word(previous) && is_word(c)) || (previous.is_whitespace() && c.is_whitespace()));
        match tokens.last_mut() {
            Some(last) if joins_previous => last.end = end,
            _ => tokens.push(start..end),
        }
        previous = Some(c);
    }
    tokens
}

// Marks the tokens of each side that belong to a longest common subsequence of both sides.
fn common_tokens(old: &[&str], new: &[&str]) -> (Vec<bool>, Vec<bool>) {
    // lengths[i][j]: length of the common subsequence of old[i..] and new[j..].
    let mut lengths = vec![vec![0_usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lengths[i][j] = if old[i] == new[j] { lengths[i + 1][j + 1] + 1 } else { lengths[i + 1][j].max(lengths[i][j + 1]) };
        }
    }

    let (mut in_old, mut in_new) = (vec![false; old.len()], vec![false; new.len()]);
    let (mut i, mut j) = (0, 0);
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            in_old[i] = true;
            in_new[j] = true;
            i += 1;
            j += 1;
        } else if lengths[i + 1][j] >= lengths[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    (in_old, in_new)
}

// Consecutive changed tokens become one range.
fn changed_ranges(tokens: &[Range<usize>], in_common: &[bool]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for (token, &common) in tokens.iter().zip(in_common) {
        if common {
            continue;
        }
        match ranges.last_mut() {
            Some(last) if last.end == token.start => last.end = token.end,
            _ => ranges.push(token.clone()),
        }
    }
    ranges
}

fn visible_chars(text: &str, ranges: &[Range<usize>]) -> usize {
    ranges.iter().map(|range| non_whitespace_chars(&text[range.clone()])).sum()
}

fn non_whitespace_chars(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

#[cfg(test)]
mod tests {
    use crate::domain::{diff_rows, DiffRow, FileDiff};

    fn diff_of(lines: &[&str]) -> Vec<DiffRow> {
        let mut file = FileDiff::new("a.txt");
        file.lines = lines.iter().map(|line| line.to_string()).collect();
        diff_rows(&file)
    }

    #[test]
    fn emphasis_marks_only_the_changed_word_of_a_modified_line() {
        let rows = diff_of(&["@@ -1 +1 @@", "-let x = foo(1);", "+let x = bar(1);"]);

        assert_eq!(&rows[1].text[rows[1].emphasis[0].clone()], "foo");
        assert_eq!(&rows[2].text[rows[2].emphasis[0].clone()], "bar");
        assert_eq!((rows[1].emphasis.len(), rows[2].emphasis.len()), (1, 1));
    }

    #[test]
    fn emphasis_skips_the_shared_prefix_and_suffix() {
        let rows = diff_of(&["@@ -1 +1 @@", "-fn compute(a: u32) -> u32 {", "+fn compute(a: u64) -> u32 {"]);

        assert_eq!(&rows[1].text[rows[1].emphasis[0].clone()], "u32");
        assert_eq!(&rows[2].text[rows[2].emphasis[0].clone()], "u64");
        assert_eq!(rows[1].emphasis.len(), 1);
    }

    #[test]
    fn emphasis_is_dropped_when_the_pair_is_too_different() {
        let rows = diff_of(&["@@ -1 +1 @@", "-foo bar baz", "+qux quux corge"]);

        assert!(rows[1].emphasis.is_empty() && rows[2].emphasis.is_empty());
    }

    #[test]
    fn emphasis_pairs_by_position_and_leaves_the_extra_rows_plain() {
        let rows = diff_of(&["@@ -1,2 +1 @@", "-let x = 1;", "-let y = 2;", "+let x = 3;"]);

        assert!(!rows[1].emphasis.is_empty());
        assert!(rows[2].emphasis.is_empty());
        assert!(!rows[3].emphasis.is_empty());
    }

    #[test]
    fn emphasis_ranges_stay_on_char_boundaries_with_multi_byte_text() {
        let rows = diff_of(&["@@ -1 +1 @@", "-let naïve = 1;", "+let naive = 1;"]);

        assert_eq!(&rows[1].text[rows[1].emphasis[0].clone()], "naïve");
        assert_eq!(&rows[2].text[rows[2].emphasis[0].clone()], "naive");
    }
}
