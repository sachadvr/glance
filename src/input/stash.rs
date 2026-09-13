use crate::domain::{Commit, Revision};

use super::oneline::is_reflog_selector;
use super::Document;

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    parse_stash_list(lines).map(Document::Log)
}

// "stash@{0}: WIP on main: 238d615 Fix the feed": a `git stash list` line, opened through `git stash show`.
fn stash_entry(line: &str) -> Option<Commit> {
    let (reference, subject) = line.split_once(": ")?;
    if !reference.starts_with("stash@{") || !is_reflog_selector(reference) {
        return None;
    }

    Some(Commit::new(Revision::Stash(reference.to_string()), subject.trim()))
}

fn parse_stash_list(lines: &[&str]) -> Option<Vec<Commit>> {
    let entries = lines.iter().filter(|line| !line.trim().is_empty()).map(|line| stash_entry(line)).collect::<Option<Vec<Commit>>>()?;

    (!entries.is_empty()).then_some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stash_list_reads_entries_opened_by_reference() {
        let entries = parse_stash_list(&["stash@{0}: WIP on main: 238d615 Fix the feed", "stash@{1}: On feature-x: promotion of attributes", ""]).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!((&entries[0].revision, entries[0].subject.as_str()), (&Revision::Stash("stash@{0}".into()), "WIP on main: 238d615 Fix the feed"));
        assert!(entries.iter().all(|entry| entry.is_stash() && entry.files.is_none()));
    }

    #[test]
    fn parse_stash_list_refuses_another_reflog() {
        assert!(parse_stash_list(&["HEAD@{0}: commit: Init"]).is_none());
        assert!(parse_stash_list(&[""]).is_none());
    }
}
