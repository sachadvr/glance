//! Reading what git pipes to its pager: one file per format, tried in the order of `DETECTORS`.

mod ansi;
mod log;
mod patch;

use crate::domain::{BlameLine, Commit, FileDiff};

pub use ansi::strip_ansi;
pub use log::parse_log;
pub use patch::parse_patch;

pub enum Document {
    Diff(Vec<FileDiff>),
    Log(Vec<Commit>),
    Blame(Vec<BlameLine>),
}

impl Document {
    fn is_empty(&self) -> bool {
        match self {
            Document::Diff(files) => files.is_empty(),
            Document::Log(commits) => commits.is_empty(),
            Document::Blame(lines) => lines.is_empty(),
        }
    }
}

type Detector = fn(&[&str]) -> Option<Document>;

// The first that recognises the text wins. A log printed with -p holds patches: the log goes before the patch.
const DETECTORS: [Detector; 2] = [log::detect, patch::detect];

/// Recognises what git piped, after its colours are stripped: `None` for what is neither (git branch, --stat, status) or is empty.
pub fn read(text: &str) -> Option<Document> {
    let plain = strip_ansi(text);
    let lines: Vec<&str> = plain.lines().collect();

    // An empty result is left to a plain pager, not handed to a later detector: the format was recognised.
    DETECTORS.iter().find_map(|detect| detect(&lines)).filter(|document| !document.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn read_leaves_what_is_neither_a_log_nor_a_patch() {
        let branch = "* main\n  feature/search\n  deadbeef\n";
        let stat = " src/a.php | 3 +-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        let status = "On branch main\nnothing to commit, working tree clean\n";

        for text in [branch, stat, status, "", "\n\n"] {
            assert!(read(text).is_none(), "{text}");
        }
    }

    #[test]
    fn an_empty_document_is_empty() {
        assert!(Document::Diff(Vec::new()).is_empty() && Document::Log(Vec::new()).is_empty() && Document::Blame(Vec::new()).is_empty());
        assert!(!Document::Diff(vec![FileDiff::new("a")]).is_empty());
    }
}
