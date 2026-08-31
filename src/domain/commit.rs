use std::rc::Rc;

use super::file::{total_stats, ChangeStats, FileDiff};

/// What a commit-like entry is opened through: `git show <hash>` or `git stash show <stash@{n}>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Revision {
    Hash(String),
    Stash(String),
}

impl Revision {
    pub fn as_str(&self) -> &str {
        match self {
            Revision::Hash(text) | Revision::Stash(text) => text,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub revision: Revision,
    pub subject: String,
    pub author: String,
    /// A date, a relative age, or a reflog selector ("HEAD@{2}"): whatever git said about when.
    pub when: String,
    /// `None` until known: a log piped without its patches leaves the files to load when the commit opens.
    pub files: Option<Rc<[FileDiff]>>,
}

impl Commit {
    pub fn new(revision: Revision, subject: impl Into<String>) -> Commit {
        Commit { revision, subject: subject.into(), author: String::new(), when: String::new(), files: None }
    }

    pub fn is_stash(&self) -> bool {
        matches!(self.revision, Revision::Stash(_))
    }

    pub fn stats(&self) -> ChangeStats {
        self.files.as_deref().map(total_stats).unwrap_or_default()
    }

    /// The facts shown under the subject: a `git log --oneline` commit has neither author nor date.
    pub fn byline(&self) -> Vec<&str> {
        [self.author.as_str(), self.when.as_str()].into_iter().filter(|fact| !fact.is_empty()).collect()
    }
}
