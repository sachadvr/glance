use std::collections::HashSet;

use crate::domain::Commit;

use super::log::{header_line, is_hash, short_hash, without_graph};
use super::Document;

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    parse_oneline_log(lines).map(Document::Log)
}

// "abc1234 (HEAD -> main, tag: v1) Fix the feed": a `git log --oneline` line, decorations dropped.
fn oneline_commit(line: &str) -> Option<Commit> {
    let (hash, subject) = header_line(line)?.split_once(' ')?;
    if !is_hash(hash) {
        return None;
    }
    let subject = subject.strip_prefix('(').and_then(|decorated| decorated.split_once(") ")).map_or(subject, |(_, rest)| rest);
    // `git reflog` puts the entry's selector first ("HEAD@{2}: checkout: moving from a to b"): it reads as
    // the entry's "when", beside the action.
    let (when, subject) = subject.split_once(": ").filter(|(selector, _)| is_reflog_selector(selector)).unwrap_or(("", subject));

    let mut commit = Commit::new(short_hash(hash), subject.trim());
    commit.when = when.to_string();
    Some(commit)
}

// "HEAD@{2}", "main@{0}", "stash@{1}".
pub(super) fn is_reflog_selector(text: &str) -> bool {
    text.split_once("@{").is_some_and(|(name, index)| !name.is_empty() && !name.contains(' ') && index.strip_suffix('}').is_some_and(|index| !index.is_empty()))
}

fn parse_oneline_log(lines: &[&str]) -> Option<Vec<Commit>> {
    let commits = lines
        .iter()
        .filter(|line| !without_graph(line).is_empty())
        .map(|line| oneline_commit(line))
        .collect::<Option<Vec<Commit>>>()?;
    // `git blame` lines start with a hash too, but repeat it on every line a commit wrote; a reflog repeats
    // hashes as well, each entry marked by its selector.
    let distinct_hashes: HashSet<&str> = commits.iter().map(|commit| commit.revision.as_str()).collect();
    let is_reflog = commits.iter().all(|commit| !commit.when.is_empty());

    (!commits.is_empty() && (is_reflog || distinct_hashes.len() == commits.len())).then_some(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn parse_oneline_log_reads_decorations_and_graph_lanes() {
        let log = "* 0123456 (HEAD -> main, tag: v1.2) Fix the feed\n|\\\n| * 89abcde (origin/x) Add (the) flag\n|/\n* fedcba9 First\n";

        let commits = parse_oneline_log(&lines(log)).unwrap();

        let summary: Vec<(&str, &str)> = commits.iter().map(|commit| (commit.revision.as_str(), commit.subject.as_str())).collect();
        assert_eq!(summary, [("0123456", "Fix the feed"), ("89abcde", "Add (the) flag"), ("fedcba9", "First")]);
        assert!(commits.iter().all(|commit| commit.files.is_none() && commit.author.is_empty()));
    }

    #[test]
    fn parse_oneline_log_reads_a_reflog_with_its_selectors_even_when_hashes_repeat() {
        let reflog = "238d6155 (HEAD -> hotfix) HEAD@{0}: cherry-pick: fix(parser): Stop dropping regions\n26ce5db4 HEAD@{1}: commit: Init hotfix\n238d6155 HEAD@{2}: checkout: moving from master to hotfix\n";

        let entries = parse_oneline_log(&lines(reflog)).unwrap();

        let read: Vec<(&str, &str, &str)> = entries.iter().map(|entry| (entry.revision.as_str(), entry.when.as_str(), entry.subject.as_str())).collect();
        assert_eq!(read, [("238d6155", "HEAD@{0}", "cherry-pick: fix(parser): Stop dropping regions"), ("26ce5db4", "HEAD@{1}", "commit: Init hotfix"), ("238d6155", "HEAD@{2}", "checkout: moving from master to hotfix")]);
    }

    #[test]
    fn parse_oneline_log_refuses_repeated_hashes_without_selectors() {
        assert!(parse_oneline_log(&["0123456 First", "0123456 Again"]).is_none());
    }

    #[test]
    fn parse_oneline_log_refuses_a_line_that_is_not_a_commit() {
        assert!(parse_oneline_log(&["0123456 First", "not a commit"]).is_none());
    }

    #[test]
    fn is_reflog_selector_wants_a_name_and_an_index() {
        assert!(is_reflog_selector("HEAD@{2}") && is_reflog_selector("stash@{0}"));
        assert!(!is_reflog_selector("@{0}") && !is_reflog_selector("HEAD@{}") && !is_reflog_selector("a b@{0}") && !is_reflog_selector("HEAD"));
    }
}
