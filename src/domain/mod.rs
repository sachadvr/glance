//! What glance shows, free of how it is read (git, a pipe) or drawn (a terminal).

mod blame;
mod commit;
mod diff;
mod file;

pub use blame::{Blame, BlameLine};
pub use commit::{Commit, Revision};
pub use diff::{diff_rows, expand_tabs, DiffRow, RowKind};
pub use file::{sort_by_directory, total_stats, ChangeStats, FileDiff, Status};
