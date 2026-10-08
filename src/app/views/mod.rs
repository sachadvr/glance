//! The screens. Each is one implementation of `View`: a new screen is a new file here.

mod blame;
mod commit_files;
mod commits;
mod diff;
mod files;
mod picker;

pub use blame::BlameView;
pub use commit_files::CommitFilesView;
pub use commits::CommitsView;
pub use diff::DiffView;
pub use files::FilesView;
pub use picker::BasePickerView;

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::command::{Command, Motion};
use crate::ui::Hit;

const BACK: &str = " ← ";

/// A cursor over a list, and the rows the list showed at its last draw: what a page key moves by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ListCursor {
    index: usize,
    page: usize,
}

impl Default for ListCursor {
    fn default() -> ListCursor {
        ListCursor { index: 0, page: 20 }
    }
}

impl ListCursor {
    fn apply(&mut self, motion: Motion, length: usize) {
        self.index = motion.apply(self.within(length), length, self.page);
    }

    /// The cursor within a list that may have shrunk since (a reload, a new base).
    fn within(&self, length: usize) -> usize {
        self.index.min(length.saturating_sub(1))
    }

    fn drawn(&mut self, area: Rect) {
        self.page = usize::from(area.height).max(1);
    }
}

/// The "←" of a screen's title: a click goes back.
fn back_hit(area: Rect) -> Hit {
    Hit { area: Rect::new(area.x, area.y, BACK.width() as u16, 1), command: Command::Back }
}
