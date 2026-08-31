use std::ops::Range;

use super::file::FileDiff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Hunk,
    Added,
    Removed,
    Context,
    Note,
}

/// A row of a diff as drawn: both sides' line numbers and the text without its +/- marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub kind: RowKind,
    pub text: String,
    /// Byte ranges of `text` that differ from the line it was paired with.
    pub emphasis: Vec<Range<usize>>,
}

impl DiffRow {
    pub fn new(kind: RowKind, old: Option<usize>, new: Option<usize>, text: impl Into<String>) -> DiffRow {
        DiffRow { old, new, kind, text: text.into(), emphasis: Vec::new() }
    }
}

// Tabs would be drawn one cell wide by the terminal backend and shift every column after them.
pub fn expand_tabs(text: &str) -> String {
    text.replace('\t', "    ")
}

// "@@ -12,4 +12,6 @@ fn x": where each side starts, combined diffs ("@@@ -a -b +c @@@") included.
fn hunk_start(header: &str) -> (usize, usize) {
    let side = |sign: char| {
        header
            .split_whitespace()
            .find_map(|token| token.strip_prefix(sign))
            .and_then(|range| range.split(',').next())
            .and_then(|start| start.parse().ok())
            .unwrap_or(0)
    };

    (side('-'), side('+'))
}

pub fn diff_rows(file: &FileDiff) -> Vec<DiffRow> {
    let (mut old, mut new) = (0, 0);
    file
        .lines
        .iter()
        .map(|line| {
            if line.starts_with("@@") {
                (old, new) = hunk_start(line);
                return DiffRow::new(RowKind::Hunk, None, None, line.clone());
            }
            let body = expand_tabs(line.get(1..).unwrap_or(""));
            match line.chars().next() {
                Some('+') => {
                    new += 1;
                    DiffRow::new(RowKind::Added, None, Some(new - 1), body)
                }
                Some('-') => {
                    old += 1;
                    DiffRow::new(RowKind::Removed, Some(old - 1), None, body)
                }
                Some(' ') => {
                    old += 1;
                    new += 1;
                    DiffRow::new(RowKind::Context, Some(old - 1), Some(new - 1), body)
                }
                _ => DiffRow::new(RowKind::Note, None, None, expand_tabs(line)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_rows_numbers_both_sides_from_the_hunk_header() {
        let mut file = FileDiff::new("a.txt");
        file.lines = ["@@ -10,3 +10,3 @@", " same", "-old", "+new", "\\ No newline at end of file"].map(String::from).to_vec();

        let rows = diff_rows(&file);

        assert_eq!(rows[1], DiffRow::new(RowKind::Context, Some(10), Some(10), "same"));
        assert_eq!((rows[2].kind, rows[2].old, rows[2].new, rows[2].text.as_str()), (RowKind::Removed, Some(11), None, "old"));
        assert_eq!((rows[3].kind, rows[3].old, rows[3].new, rows[3].text.as_str()), (RowKind::Added, None, Some(11), "new"));
        assert_eq!(rows[4].kind, RowKind::Note);
    }

    #[test]
    fn diff_rows_expands_tabs() {
        let mut file = FileDiff::new("a.txt");
        file.lines = ["@@ -1 +1 @@", "+\tx"].map(String::from).to_vec();
        assert_eq!(diff_rows(&file)[1].text, "    x");
    }
}
