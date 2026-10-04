//! Drawing glance in a terminal frame: the primitives every view is built from.

mod diff;
mod list;
mod row;
mod stat;
mod text;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::app::command::Command;
use crate::theme::Theme;

pub use diff::{colored_spans, diff_lines, DIFF_GUTTER_WIDTH};
pub use list::{draw_list, every_row, tests_separator};
pub use row::Row;
pub use stat::{stat_line, stat_spans, status_color, StatColumns};
pub use text::{fit_end, fit_start, padded, spans_width};

/// A clickable area, recorded at each draw: a click inside it runs `command`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub area: Rect,
    pub command: Command,
}

pub fn grey(text: impl Into<String>, theme: &Theme) -> Span<'static> {
    Span::styled(text.into(), Style::new().fg(theme.grey))
}
