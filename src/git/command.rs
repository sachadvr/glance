use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::{sort_by_directory, FileDiff, Revision};
use crate::input::parse_patch;

/// What went wrong running git. The interface words it in its language; git's own messages stay as git wrote them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitError {
    /// Counting from the main line, when the repository has none.
    NoMainline,
    /// git could not run, or said why it failed.
    Failed(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitError::NoMainline => formatter.write_str("no main branch found"),
            GitError::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GitError {}

pub type GitResult<T> = Result<T, GitError>;

/// git run in one directory. As a pager, glance runs where git did: `Git::here()` is enough to load a commit's files.
#[derive(Clone, Debug)]
pub struct Git {
    root: PathBuf,
}

pub(super) fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

impl Git {
    pub fn here() -> Git {
        Git::at(".")
    }

    pub(super) fn at(root: impl Into<PathBuf>) -> Git {
        Git { root: root.into() }
    }

    pub(super) fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.root).args(["-c", "core.quotePath=false"]).args(args);
        command
    }

    pub(super) fn run(&self, args: &[&str]) -> GitResult<String> {
        Ok(String::from_utf8_lossy(&self.run_bytes(args)?).into_owned())
    }

    // What git printed as it printed it: a file's content may not be text.
    fn run_bytes(&self, args: &[&str]) -> GitResult<Vec<u8>> {
        let output = self.command(args).output().map_err(|error| GitError::Failed(error.to_string()))?;
        if !output.status.success() {
            return Err(GitError::Failed(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }

        Ok(output.stdout)
    }

    /// A file as `revision` left it, or as it is in the working tree without one. `path` is from the repository's root,
    /// as a patch names it.
    pub fn content(&self, revision: Option<&Revision>, path: &str) -> GitResult<Vec<u8>> {
        if let Some(revision) = revision {
            return self.run_bytes(&["show", "--no-textconv", &format!("{}:{path}", revision.as_str())]);
        }
        // As a pager glance may run from a subdirectory, while the patch names the file from the root.
        let root = self.run(&["rev-parse", "--show-toplevel"])?;
        fs::read(Path::new(root.trim()).join(path)).map_err(|error| GitError::Failed(error.to_string()))
    }

    // A branch opens on its tip commit; "--" keeps a branch named like a file a revision. `git show` of a stash
    // draws it as a merge, against both its parents; `git stash show` gives the change it saved, untracked files included.
    pub fn files_of(&self, revision: &Revision) -> GitResult<Vec<FileDiff>> {
        let patch = match revision {
            Revision::Hash(name) | Revision::Branch(name) => self.run(&["show", "--no-color", "--no-ext-diff", "-M", "--format=", name, "--"])?,
            Revision::Stash(reference) => self.run(&["stash", "show", "--no-color", "--no-ext-diff", "-p", "-M", "--include-untracked", reference])?,
        };
        let mut files = parse_patch(&lines(&patch));
        sort_by_directory(&mut files);

        Ok(files)
    }

    pub fn subject(&self, hash: &str) -> GitResult<String> {
        Ok(self.run(&["log", "-1", "--no-color", "--format=%s", hash])?.trim().to_string())
    }

    pub fn fetch(&self) -> GitResult<()> {
        self.run(&["fetch", "--quiet", "origin"]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_is_sendable_to_a_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<Git>();
    }

    #[test]
    fn run_reports_what_git_wrote_on_stderr() {
        let error = Git::at("/nonexistent/glance").run(&["status"]).unwrap_err();
        assert!(error.to_string().starts_with("fatal:"), "{error}");
    }
}
