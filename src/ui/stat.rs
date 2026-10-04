use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::grey;
use super::text::{fit_start, padded};
use crate::domain::{ChangeStats, FileDiff, Status};
use crate::theme::Theme;

const MAX_BAR: usize = 25;
// " M " before the path, " │ " after it, then "1234 " before the bar.
const STAT_CHROME_WIDTH: usize = 3 + 3 + 5;

pub fn status_color(status: Status, theme: &Theme) -> Color {
    match status {
        Status::Modified => theme.blue,
        Status::Added => theme.green,
        Status::Deleted => theme.red,
        Status::Renamed => theme.accent,
        Status::Untracked => theme.cyan,
    }
}

/// " +12 −3", each side left out when it is zero.
pub fn stat_spans(stats: ChangeStats, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if stats.insertions > 0 {
        spans.push(Span::styled(format!(" +{}", stats.insertions), Style::new().fg(theme.green)));
    }
    if stats.deletions > 0 {
        spans.push(Span::styled(format!(" −{}", stats.deletions), Style::new().fg(theme.red)));
    }
    spans
}

/// The widths shared by the stat rows of a list of files, so their bars line up and scale together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatColumns {
    pub path_width: usize,
    pub bar_width: usize,
    /// The most lines a file of the list changed: its bar takes the full `bar_width`.
    pub largest: usize,
}

impl StatColumns {
    /// Columns for `files` drawn in `width` cells, after `indent` cells of margin.
    pub fn fit(files: &[FileDiff], width: usize, indent: usize) -> StatColumns {
        let largest = files.iter().map(|file| file.stats.changed()).max().unwrap_or(0);
        let bar_width = MAX_BAR.min(largest).min(width / 3);
        let path_width = width.saturating_sub(indent + STAT_CHROME_WIDTH + bar_width);
        StatColumns { path_width, bar_width, largest }
    }

    // Small counts are drawn as is; past the bar's width they shrink in proportion, a change never vanishing.
    fn scaled(self, count: usize) -> usize {
        if self.largest <= self.bar_width || count == 0 {
            return count;
        }
        ((count * self.bar_width) as f64 / self.largest as f64).round().max(1.0) as usize
    }
}

/// One row per file, as `git log --stat` draws it: "path │ count +++−−".
pub fn stat_line(file: &FileDiff, columns: StatColumns, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(file.status.letter().to_string(), Style::new().fg(status_color(file.status, theme)).bold()),
        Span::raw(" "),
        Span::raw(padded(fit_start(&file.path, columns.path_width), columns.path_width)),
        grey(" │ ", theme),
        Span::raw(format!("{:>4} ", file.stats.changed())),
        Span::styled("+".repeat(columns.scaled(file.stats.insertions)), Style::new().fg(theme.green)),
        Span::styled("−".repeat(columns.scaled(file.stats.deletions)), Style::new().fg(theme.red)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeSelection;

    fn file(path: &str, insertions: usize, deletions: usize) -> FileDiff {
        let mut file = FileDiff::new(path);
        file.stats = ChangeStats { insertions, deletions };
        file
    }

    #[test]
    fn fit_caps_the_bar_and_gives_the_rest_to_the_path() {
        let columns = StatColumns::fit(&[file("a", 100, 50), file("b", 1, 0)], 90, 3);

        assert_eq!(columns, StatColumns { path_width: 90 - 3 - 11 - 25, bar_width: 25, largest: 150 });
    }

    #[test]
    fn fit_keeps_the_bar_to_a_third_of_a_narrow_width() {
        assert_eq!(StatColumns::fit(&[file("a", 100, 0)], 30, 0).bar_width, 10);
    }

    #[test]
    fn scaled_draws_small_counts_as_is_and_shrinks_large_ones_without_losing_any() {
        let small = StatColumns::fit(&[file("a", 4, 2)], 90, 0);
        assert_eq!((small.scaled(4), small.scaled(2)), (4, 2));

        let large = StatColumns::fit(&[file("a", 100, 0), file("b", 1, 0)], 90, 0);
        assert_eq!((large.scaled(100), large.scaled(1), large.scaled(0)), (25, 1, 0));
    }

    #[test]
    fn stat_spans_leave_out_a_zero_side() {
        let theme = ThemeSelection::default().current();
        let spans = stat_spans(ChangeStats { insertions: 3, deletions: 0 }, theme);

        assert_eq!(spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>(), [" +3"]);
    }
}
