use std::rc::Rc;

use crate::domain::{sort_by_directory, Blame, Commit, FileDiff};
use crate::git::{Base, Git, GitResult, Repo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchState {
    Idle,
    Running,
    Done,
    Failed,
}

/// Where what is shown comes from.
pub enum Source {
    /// Fed by git through `core.pager`: what it piped is all there is, and the header names the command.
    Pager { command: String },
    Repo(Repo),
}

/// What glance shows, and the git that can load more of it. Screens read it; changes go through its methods.
pub struct Session {
    pub source: Source,
    pub files: Rc<[FileDiff]>,
    pub commits: Vec<Commit>,
    pub blame: Blame,
    pub base: Base,
    pub fetch: FetchState,
}

impl Session {
    pub fn new(source: Source) -> Session {
        Session { source, files: Rc::from([]), commits: Vec::new(), blame: Blame::default(), base: Base::Worktree, fetch: FetchState::Idle }
    }

    pub fn repo(&self) -> Option<&Repo> {
        match &self.source {
            Source::Repo(repo) => Some(repo),
            Source::Pager { .. } => None,
        }
    }

    // As a pager, glance runs where git did, inside the repository: that is enough to load a commit's files.
    pub fn git(&self) -> Git {
        self.repo().map_or_else(Git::here, |repo| repo.git.clone())
    }

    pub fn set_files(&mut self, mut files: Vec<FileDiff>) {
        sort_by_directory(&mut files);
        self.files = Rc::from(files);
    }

    /// Reads the branch again. A pager has nothing to read again: what git piped stays.
    pub fn reload(&mut self) -> GitResult<()> {
        let Some(repo) = self.repo() else {
            return Ok(());
        };
        let commits = repo.commits()?;
        let files = repo.files(&self.base)?;
        self.commits = commits;
        self.set_files(files);
        Ok(())
    }

    /// Counts the files from `base`; when git cannot, the base stays what it was rather than fail every reload after.
    pub fn set_base(&mut self, base: Base) -> GitResult<()> {
        let previous = std::mem::replace(&mut self.base, base);
        self.reload().inspect_err(|_| self.base = previous)
    }

    /// The files of a commit, asked of git the first time and kept for the next.
    pub fn commit_files(&mut self, index: usize) -> GitResult<Rc<[FileDiff]>> {
        let git = self.git();
        let commit = &mut self.commits[index];
        if let Some(files) = &commit.files {
            return Ok(Rc::clone(files));
        }
        let files: Rc<[FileDiff]> = Rc::from(git.files_of(&commit.revision)?);
        commit.files = Some(Rc::clone(&files));
        Ok(files)
    }
}
