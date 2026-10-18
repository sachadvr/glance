//! Reading what git pipes to its pager: one file per format, tried in the order of `DETECTORS`.

mod ansi;
mod blame;
mod branch;
mod log;
mod oneline;
mod patch;
mod stash;
mod status;

use crate::domain::{BlameLine, Commit, FileDiff};

pub use ansi::strip_ansi;
pub use log::parse_log;
pub use patch::parse_patch;

pub enum Document {
    Diff(Vec<FileDiff>),
    Log(Vec<Commit>),
    Blame(Vec<BlameLine>),
    /// What `git status` lists, paths as git printed them (relative to where it ran), without patches.
    Status(Vec<FileDiff>),
}

impl Document {
    fn is_empty(&self) -> bool {
        match self {
            Document::Diff(files) | Document::Status(files) => files.is_empty(),
            Document::Log(commits) => commits.is_empty(),
            Document::Blame(lines) => lines.is_empty(),
        }
    }
}

type Detector = fn(&[&str]) -> Option<Document>;

// The first that recognises the text wins. A log printed with -p holds patches, and two blame lines of
// distinct commits read as a oneline log: the medium log goes before the patch, the blame before the oneline log.
// A graph oneline log starts its lines with "* " as a branch list does: the oneline log goes before the branches.
// `git status -v` follows the status with its patch: the strict status refuses it and the patch takes it.
const DETECTORS: [Detector; 7] = [log::detect, blame::detect, oneline::detect, stash::detect, branch::detect, status::detect, patch::detect];

/// Recognises what git piped, after its colours are stripped: `None` for what is none of its formats (--stat) or is empty.
pub fn read(text: &str) -> Option<Document> {
    let plain = strip_ansi(text);
    let lines: Vec<&str> = plain.lines().collect();

    // An empty result is left to a plain pager, not handed to a later detector: the format was recognised.
    DETECTORS.iter().find_map(|detect| detect(&lines)).filter(|document| !document.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Revision;

    #[test]
    fn read_tells_a_log_from_a_diff_and_strips_colours() {
        let log = format!("\x1b[33mcommit 0123456789abcdef\x1b[m (HEAD -> main)\nAuthor: Grace H <grace@example.com>\nDate:   2 days ago\n\n    Fix the feed\n\n{}", patch::DIFF);

        let Some(Document::Log(commits)) = read(&log) else { panic!("a log") };
        assert_eq!((commits[0].revision.as_str(), commits[0].files.as_deref().map(<[_]>::len)), ("012345678", Some(3)));
        assert!(matches!(read(patch::DIFF), Some(Document::Diff(files)) if files.len() == 3));
    }

    #[test]
    fn read_sorts_the_files_of_a_diff_by_directory() {
        let Some(Document::Diff(files)) = read("diff --git a/tests/ATest.php b/tests/ATest.php\ndiff --git a/src/a.php b/src/a.php\n") else { panic!("a diff") };

        assert_eq!(files[0].path, "src/a.php");
    }

    #[test]
    fn read_takes_a_blame_before_a_oneline_log() {
        let blame = "7bfd4f45 (Linus 2024-08-21 23:03:02 +0200 1) <?php\n238d6155 (Grace Hopper 2026-10-16 18:01:20 +0200 2) return;\n";

        assert!(matches!(read(blame), Some(Document::Blame(lines)) if lines.len() == 2));
    }

    #[test]
    fn read_reads_oneline_logs_reflogs_and_stash_lists_as_logs() {
        let Some(Document::Log(oneline)) = read("0123456 First\n89abcde Second\n") else { panic!("a oneline log") };
        let Some(Document::Log(reflog)) = read("238d6155 HEAD@{0}: commit: Init\n238d6155 HEAD@{1}: checkout: moving\n") else { panic!("a reflog") };
        let Some(Document::Log(stashes)) = read("stash@{0}: WIP on main: 238d615 Fix\n") else { panic!("a stash list") };

        assert_eq!((oneline.len(), reflog.len()), (2, 2));
        assert_eq!(stashes[0].revision, Revision::Stash("stash@{0}".into()));
    }

    #[test]
    fn read_reads_a_branch_list_as_branches_to_open() {
        let Some(Document::Log(branches)) = read("* main\n  feature/search\n  deadbeef\n") else { panic!("a branch list") };

        assert_eq!(branches.iter().map(|branch| branch.revision.clone()).collect::<Vec<_>>(), ["main", "feature/search", "deadbeef"].map(|name| Revision::Branch(name.into())));
        assert!(branches[0].current && !branches[1].current);
    }

    #[test]
    fn read_reads_a_long_and_a_short_status_with_colours() {
        let long = "On branch main\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n\t\x1b[31mmodified:   src/a.rs\x1b[m\n\nUntracked files:\n\t\x1b[31mnotes.txt\x1b[m\n";
        let short = "## main\n\x1b[31m M\x1b[m src/a.rs\n\x1b[31m??\x1b[m notes.txt\n";

        for text in [long, short] {
            let Some(Document::Status(files)) = read(text) else { panic!("a status: {text}") };
            assert_eq!(files.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), ["src/a.rs", "notes.txt"]);
        }
    }

    #[test]
    fn read_takes_a_graph_oneline_log_before_a_branch_list_and_a_verbose_status_as_its_patch() {
        let verbose_status = format!("On branch main\nChanges to be committed:\n\tmodified:   src/a.php\n\n{}", patch::DIFF);

        assert!(matches!(read("* 0123456 (HEAD -> main) First\n* 89abcde Second\n"), Some(Document::Log(commits)) if !commits[0].is_branch()));
        assert!(matches!(read(&verbose_status), Some(Document::Diff(files)) if files.len() == 3));
    }

    #[test]
    fn read_leaves_a_stat_a_clean_status_and_nothing() {
        let stat = " src/a.php | 3 +-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        let clean_status = "On branch main\nnothing to commit, working tree clean\n";

        for text in [stat, clean_status, "## main\n", "", "\n\n"] {
            assert!(read(text).is_none(), "{text}");
        }
    }

    #[test]
    fn an_empty_document_is_empty() {
        assert!(Document::Diff(Vec::new()).is_empty() && Document::Log(Vec::new()).is_empty() && Document::Blame(Vec::new()).is_empty() && Document::Status(Vec::new()).is_empty());
        assert!(!Document::Diff(vec![FileDiff::new("a")]).is_empty());
    }
}
