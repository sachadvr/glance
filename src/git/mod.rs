//! Running git: loading a commit's files as a pager, or the branch glance was opened on.

mod command;
mod repo;

pub use command::{Git, GitError, GitResult};
pub use repo::{Base, Repo};
