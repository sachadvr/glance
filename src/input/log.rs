use std::rc::Rc;

use crate::domain::{sort_by_directory, Commit, Revision};

use super::patch::{file_start_path, parse_patch};
use super::Document;

// `git log --graph` draws its lanes left of every line with these.
const GRAPH: [char; 6] = ['*', '|', '/', '\\', '_', ' '];

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    lines.iter().any(|line| commit_hash(line).is_some()).then(|| Document::Log(parse_log(lines)))
}

pub(super) fn without_graph(line: &str) -> &str {
    line.trim_start_matches(GRAPH)
}

// A commit starts at column 0, or right after graph lanes holding its node ("| * commit x"). An indented
// line (a message, a patch's context, a docblock's " * ") never starts one.
pub(super) fn header_line(line: &str) -> Option<&str> {
    let rest = without_graph(line);
    let lanes = &line[..line.len() - rest.len()];
    let is_node = lanes.is_empty() || (!lanes.starts_with(' ') && lanes.contains('*'));

    is_node.then_some(rest)
}

pub(super) fn is_hash(text: &str) -> bool {
    text.len() >= 7 && text.len() <= 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// Nine characters tell commits apart in any repository a pager is used in.
pub(super) fn short_hash(hash: &str) -> Revision {
    Revision::Hash(hash.chars().take(9).collect())
}

fn commit_hash(line: &str) -> Option<&str> {
    let hash = header_line(line)?.strip_prefix("commit ")?.split(' ').next()?;

    is_hash(hash).then_some(hash)
}

pub fn parse_log(lines: &[&str]) -> Vec<Commit> {
    // A log printed with -p carries every commit's patch: one without files really changed none (a merge).
    let has_patches = lines.iter().any(|line| file_start_path(line).is_some());
    let starts: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| commit_hash(line).is_some()).map(|(index, _)| index).collect();

    starts
        .iter()
        .enumerate()
        .map(|(position, &start)| {
            let end = starts.get(position + 1).copied().unwrap_or(lines.len());
            parse_commit(&lines[start..end], has_patches)
        })
        .collect()
}

fn header_field<'a>(header: &[&'a str], name: &str) -> &'a str {
    header.iter().find_map(|line| without_graph(line).strip_prefix(name)).map_or("", str::trim)
}

fn parse_commit(lines: &[&str], has_patches: bool) -> Commit {
    let hash = commit_hash(lines[0]).unwrap_or_default();
    // In a graph, the message keeps its four-space indent past the lanes drawn left of the commit line.
    let lanes = lines[0].len() - without_graph(lines[0]).len();
    let diff_start = lines.iter().position(|line| file_start_path(line).is_some()).unwrap_or(lines.len());
    let header = &lines[1..diff_start];
    let subject = header.iter().filter_map(|line| line.get(lanes..)).find(|line| line.starts_with("    ") && !line.trim().is_empty()).map_or("", str::trim);

    let mut commit = Commit::new(short_hash(hash), subject);
    commit.author = header_field(header, "Author:").split(" <").next().unwrap_or("").to_string();
    commit.when = header_field(header, "Date:").to_string();
    commit.files = has_patches.then(|| {
        let mut files = parse_patch(&lines[diff_start..]);
        sort_by_directory(&mut files);
        Rc::from(files)
    });
    commit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn parse_log_reads_a_log_with_its_patches_sorted() {
        let log = "commit 0123456789abcdef (HEAD -> main)\nAuthor: Grace H <grace@example.com>\nDate:   2 days ago\n\n    Fix the feed\n\ndiff --git a/tests/ATest.php b/tests/ATest.php\n@@ -1 +1 @@\n+x\ndiff --git a/src/a.php b/src/a.php\n@@ -1 +1 @@\n-y\n";

        let commits = parse_log(&lines(log));

        assert_eq!(commits.len(), 1);
        assert_eq!((commits[0].revision.as_str(), commits[0].subject.as_str(), commits[0].author.as_str(), commits[0].when.as_str()), ("012345678", "Fix the feed", "Grace H", "2 days ago"));
        let paths: Vec<&str> = commits[0].files.as_deref().unwrap().iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["src/a.php", "tests/ATest.php"]);
    }

    #[test]
    fn parse_log_gives_a_merge_of_a_log_with_patches_no_files_rather_than_unknown_ones() {
        let log = "commit 1111111aaaa\nMerge: 2222222 3333333\nAuthor: Lea <lea@example.com>\n\n    Merge branch 'x'\n\ncommit 3333333cccc\nAuthor: Lea <lea@example.com>\n\n    Add\n\ndiff --git a/a b/a\n@@ -1 +1 @@\n+x\n";

        let commits = parse_log(&lines(log));

        assert_eq!(commits[0].files.as_deref().map(<[_]>::len), Some(0));
        assert_eq!(commits[1].files.as_deref().map(<[_]>::len), Some(1));
    }

    #[test]
    fn parse_log_reads_a_log_without_patches_and_leaves_its_files_to_load() {
        let log = "commit 0123456789abcdef (HEAD -> main)\nAuthor: Grace H <grace@example.com>\nDate:   2 days ago\n\n    Fix the feed\n\n    Body.\n\ncommit fedcba9876543210\nMerge: 1234567 89abcde\nAuthor: Lea <lea@example.com>\nDate:   3 days ago\n\n    Merge branch 'x'\n";

        let commits = parse_log(&lines(log));

        let summary: Vec<(&str, &str, &str, bool)> =
            commits.iter().map(|commit| (commit.revision.as_str(), commit.subject.as_str(), commit.author.as_str(), commit.files.is_some())).collect();
        assert_eq!(summary, [("012345678", "Fix the feed", "Grace H", false), ("fedcba987", "Merge branch 'x'", "Lea", false)]);
    }

    #[test]
    fn parse_log_reads_a_graph_log_past_its_lanes() {
        let log = "*   commit 1111111aaaa (HEAD -> main)
|\\  Merge: 2222222 3333333
| | Author: Grace H <grace@example.com>
| | Date:   1 day ago
| |
| |     Merge branch 'feature'
| |
| * commit 3333333cccc
|/  Author: Lea <lea@example.com>
|   Date:   2 days ago
|
|       Add the feature
|
* commit 2222222bbbb
  Author: Grace H <grace@example.com>
  Date:   3 days ago

      * First
";

        let commits = parse_log(&lines(log));

        let summary: Vec<(&str, &str, &str, &str)> =
            commits.iter().map(|commit| (commit.revision.as_str(), commit.subject.as_str(), commit.author.as_str(), commit.when.as_str())).collect();
        assert_eq!(
            summary,
            [
                ("1111111aa", "Merge branch 'feature'", "Grace H", "1 day ago"),
                ("3333333cc", "Add the feature", "Lea", "2 days ago"),
                ("2222222bb", "* First", "Grace H", "3 days ago"),
            ]
        );
    }

    #[test]
    fn header_line_ignores_an_indented_line_that_looks_like_a_commit() {
        assert_eq!(header_line("    commit 0123456 in a message"), None);
        assert_eq!(header_line(" * commit 0123456 in a docblock"), None);
        assert_eq!(header_line("| * commit 0123456"), Some("commit 0123456"));
    }

    #[test]
    fn is_hash_wants_seven_to_sixty_four_hex_digits() {
        assert!(is_hash("0123456") && is_hash(&"a".repeat(64)));
        assert!(!is_hash("012345") && !is_hash(&"a".repeat(65)) && !is_hash("deadbeg"));
    }
}
