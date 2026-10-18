use crate::domain::{FileDiff, Status};

use super::Document;

// The first line of a long `git status`: where HEAD is.
const HEAD_STATES: [&str; 5] = ["On branch ", "HEAD detached ", "Not currently on any branch.", "interactive rebase in progress; onto ", "rebase in progress; onto "];

// What a long status says around its sections; its hints are indented by two spaces, a rebase's todo by three.
const NARRATION: [&str; 20] = [
    "Your branch ",
    "and have ",
    "  (",
    "   ",
    "No commits yet",
    "Initial commit",
    "You have unmerged paths.",
    "You are currently ",
    "You are in ",
    "All conflicts fixed",
    "Last command",
    "Next command",
    "No commands ",
    "It took ",
    "nothing to commit",
    "nothing added to commit",
    "no changes added to commit",
    "Untracked files not listed",
    "Your stash currently has ",
    "The current patch is empty.",
];

// "\tnew file:   src/a.rs" in the changes sections, "\tboth modified:   src/a.rs" among unmerged paths.
const CHANGE_KINDS: [(&str, Status); 15] = [
    ("modified:", Status::Modified),
    ("new file:", Status::Added),
    ("deleted:", Status::Deleted),
    ("renamed:", Status::Renamed),
    ("copied:", Status::Added),
    ("typechange:", Status::Modified),
    ("unknown:", Status::Modified),
    ("unmerged:", Status::Modified),
    ("both modified:", Status::Modified),
    ("both added:", Status::Added),
    ("both deleted:", Status::Deleted),
    ("added by us:", Status::Added),
    ("added by them:", Status::Added),
    ("deleted by us:", Status::Deleted),
    ("deleted by them:", Status::Deleted),
];

// What git adds after a submodule's path: "sub (new commits, modified content)".
const SUBMODULE_NOTES: [&str; 3] = ["new commits", "modified content", "untracked content"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Changes,
    Untracked,
    Ignored,
}

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    parse_long_status(lines).or_else(|| parse_short_status(lines)).map(Document::Status)
}

fn section(line: &str) -> Option<Section> {
    match line {
        "Changes to be committed:" | "Changes not staged for commit:" | "Unmerged paths:" => Some(Section::Changes),
        "Untracked files:" => Some(Section::Untracked),
        "Ignored files:" => Some(Section::Ignored),
        _ => None,
    }
}

fn parse_long_status(lines: &[&str]) -> Option<Vec<FileDiff>> {
    let mut content = lines.iter().filter(|line| !line.trim().is_empty());
    let first = content.next()?;
    if !HEAD_STATES.iter().any(|state| first.starts_with(state)) {
        return None;
    }
    let mut current = None;
    let mut files = Vec::new();
    for line in content {
        if let Some(entry) = line.strip_prefix('\t') {
            match current? {
                Section::Changes => add_once(&mut files, change_entry(entry)?),
                Section::Untracked => add_once(&mut files, file_entry(Status::Untracked, entry, false)?),
                Section::Ignored => {}
            }
            continue;
        }
        if let Some(next) = section(line) {
            current = Some(next);
            continue;
        }
        if !NARRATION.iter().chain(&HEAD_STATES).any(|start| line.starts_with(start)) {
            return None;
        }
    }

    Some(files)
}

// A file both staged and changed again is listed in two sections; the list shows it once.
fn add_once(files: &mut Vec<FileDiff>, file: FileDiff) {
    if !files.iter().any(|listed| listed.path == file.path) {
        files.push(file);
    }
}

fn change_entry(entry: &str) -> Option<FileDiff> {
    let (kind, status) = CHANGE_KINDS.iter().find(|(kind, _)| entry.starts_with(kind))?;
    let paths = without_submodule_notes(entry[kind.len()..].trim_start());

    file_entry(*status, paths, matches!(*kind, "renamed:" | "copied:"))
}

fn without_submodule_notes(paths: &str) -> &str {
    let Some((path, notes)) = paths.strip_suffix(')').and_then(|rest| rest.rsplit_once(" (")) else {
        return paths;
    };
    if notes.split(", ").all(|note| SUBMODULE_NOTES.contains(&note)) { path } else { paths }
}

// "M  src/a.rs", "?? notes.txt", "R  old.rs -> new.rs", after an optional "## main...origin/main [ahead 1]".
fn parse_short_status(lines: &[&str]) -> Option<Vec<FileDiff>> {
    let mut content = lines.iter().filter(|line| !line.trim().is_empty()).peekable();
    content.peek()?;
    content.next_if(|line| line.starts_with("## "));
    let mut files = Vec::new();
    for line in content {
        let (code, paths) = (line.get(..2)?, line.get(3..)?);
        if line.as_bytes()[2] != b' ' || paths.is_empty() {
            return None;
        }
        // `--ignored` lists what git leaves out: not a change.
        if code == "!!" {
            continue;
        }
        files.push(file_entry(short_status(code)?, paths, code.contains(['R', 'C']))?);
    }

    Some(files)
}

// Either column may hold the change, staged or not; a conflict ("UU", "AA") reads as a modification.
fn short_status(code: &str) -> Option<Status> {
    if code == "??" {
        return Some(Status::Untracked);
    }
    if code == "  " || !code.chars().all(|letter| " MTADRCU".contains(letter)) {
        return None;
    }
    let status = if code.contains('U') || code == "AA" {
        Status::Modified
    } else if code.contains('R') {
        Status::Renamed
    } else if code.contains(['A', 'C']) {
        Status::Added
    } else if code.contains('D') {
        Status::Deleted
    } else {
        Status::Modified
    };
    Some(status)
}

// A rename or a copy names both sides, "old -> new"; the copy keeps where it came from, as a rename does.
fn file_entry(status: Status, paths: &str, has_origin: bool) -> Option<FileDiff> {
    let (old_path, path) = if has_origin { split_origin(paths).map(|(old, new)| (Some(unquoted(old)), new))? } else { (None, paths) };
    if path.is_empty() {
        return None;
    }

    let mut file = FileDiff::new(unquoted(path));
    file.status = status;
    file.old_path = old_path;
    Some(file)
}

fn split_origin(paths: &str) -> Option<(&str, &str)> {
    let old_end = if paths.starts_with('"') { closing_quote(paths)? + 1 } else { paths.find(" -> ")? };
    let new = paths[old_end..].strip_prefix(" -> ")?;

    Some((&paths[..old_end], new))
}

fn closing_quote(quoted: &str) -> Option<usize> {
    let bytes = quoted.as_bytes();
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

// "\"caf\\303\\251 a.txt\"" -> "café a.txt": git quotes a path holding a quote, a control character, a space (in the
// short format) or, under core.quotePath, any non-ASCII byte, written in octal.
fn unquoted(path: &str) -> String {
    let Some(inside) = path.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) else {
        return path.to_string();
    };
    let mut bytes = Vec::with_capacity(inside.len());
    let mut rest = inside.bytes();
    while let Some(byte) = rest.next() {
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        match rest.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(digit @ b'0'..=b'7') => {
                let octal = [digit, rest.next().unwrap_or(b'0'), rest.next().unwrap_or(b'0')];
                bytes.push(octal.iter().fold(0u8, |value, digit| value.wrapping_mul(8).wrapping_add(digit.wrapping_sub(b'0'))));
            }
            Some(escaped) => bytes.push(escaped),
            None => {}
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    fn summary(files: &[FileDiff]) -> Vec<(&str, Status, Option<&str>)> {
        files.iter().map(|file| (file.path.as_str(), file.status, file.old_path.as_deref())).collect()
    }

    const LONG: &str = "On branch main
Your branch is ahead of 'origin/main' by 1 commit.
  (use \"git push\" to publish your local commits)

Changes to be committed:
  (use \"git restore --staged <file>...\" to unstage)
\tmodified:   src/feed.rs
\tnew file:   src/parser.rs
\trenamed:    old.rs -> src/new.rs

Changes not staged for commit:
  (use \"git add <file>...\" to update what will be committed)
  (use \"git restore <file>...\" to discard changes in working directory)
\tmodified:   src/feed.rs
\tdeleted:    ../README.md
\tmodified:   vendor/lib (new commits, modified content)

Untracked files:
  (use \"git add <file>...\" to include in what will be committed)
\tnotes/
\tété.txt

";

    #[test]
    fn parse_long_status_reads_every_section_once_per_file() {
        let files = parse_long_status(&lines(LONG)).unwrap();

        assert_eq!(
            summary(&files),
            [
                ("src/feed.rs", Status::Modified, None),
                ("src/parser.rs", Status::Added, None),
                ("src/new.rs", Status::Renamed, Some("old.rs")),
                ("../README.md", Status::Deleted, None),
                ("vendor/lib", Status::Modified, None),
                ("notes/", Status::Untracked, None),
                ("été.txt", Status::Untracked, None),
            ]
        );
        assert!(files.iter().all(|file| file.lines.is_empty()));
    }

    #[test]
    fn parse_long_status_reads_a_detached_head_a_rebase_and_unmerged_paths() {
        let rebase = "interactive rebase in progress; onto 0123456
Last command done (1 command done):
   pick 89abcde Fix the feed
No commands remaining.
You are currently rebasing branch 'main' on '0123456'.
  (fix conflicts and then run \"git rebase --continue\")

Unmerged paths:
  (use \"git restore --staged <file>...\" to unstage)
\tboth modified:   src/feed.rs
\tdeleted by them: src/old.rs

no changes added to commit (use \"git add\" and/or \"git commit -a\")
";
        let detached = "HEAD detached at 0123456\nnothing to commit, working tree clean\n";

        assert_eq!(summary(&parse_long_status(&lines(rebase)).unwrap()), [("src/feed.rs", Status::Modified, None), ("src/old.rs", Status::Deleted, None)]);
        assert!(parse_long_status(&lines(detached)).unwrap().is_empty());
    }

    #[test]
    fn parse_long_status_refuses_an_unknown_line_or_an_entry_outside_a_section() {
        assert!(parse_long_status(&lines("On branch main\nsomething else\n")).is_none());
        assert!(parse_long_status(&lines("On branch main\n\tmodified:   a.rs\n")).is_none());
        assert!(parse_long_status(&lines("On branch main\nChanges to be committed:\n\tchanged:   a.rs\n")).is_none());
        assert!(parse_long_status(&lines("Sur la branche main\n")).is_none());
        assert!(parse_long_status(&lines("On branch main\nChanges to be committed:\n\tmodified:   a.rs\n\ndiff --git a/a.rs b/a.rs\n")).is_none());
    }

    #[test]
    fn parse_short_status_reads_both_columns_renames_and_the_branch_line() {
        let short = "## main...origin/main [ahead 1]\nM  src/feed.rs\n M src/parser.rs\nAM src/new.rs\nR  old.rs -> src/moved.rs\nUU src/conflict.rs\n D gone.rs\n?? notes/\n!! target/\n";

        let files = parse_short_status(&lines(short)).unwrap();

        assert_eq!(
            summary(&files),
            [
                ("src/feed.rs", Status::Modified, None),
                ("src/parser.rs", Status::Modified, None),
                ("src/new.rs", Status::Added, None),
                ("src/moved.rs", Status::Renamed, Some("old.rs")),
                ("src/conflict.rs", Status::Modified, None),
                ("gone.rs", Status::Deleted, None),
                ("notes/", Status::Untracked, None),
            ]
        );
    }

    #[test]
    fn parse_short_status_unquotes_the_paths_git_quoted() {
        let short = "?? \"a b.txt\"\nR  \"old \\\"x\\\".rs\" -> \"caf\\303\\251.rs\"\n";

        assert_eq!(summary(&parse_short_status(&lines(short)).unwrap()), [("a b.txt", Status::Untracked, None), ("café.rs", Status::Renamed, Some("old \"x\".rs"))]);
    }

    #[test]
    fn parse_short_status_of_a_clean_branch_is_empty() {
        assert!(parse_short_status(&lines("## main...origin/main\n")).unwrap().is_empty());
    }

    #[test]
    fn parse_short_status_refuses_what_is_not_a_short_status() {
        for text in [
            " src/a.php | 3 +-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n",
            "* main\n  feature\n",
            "0123456 First\n",
            "M\tsrc/a.rs\n",
            "? src/a.rs\n",
            "XY src/a.rs\n",
            "M  \n",
            "M  src/a.rs\n## main\n",
            "R  no-arrow.rs\n",
            "",
        ] {
            assert!(parse_short_status(&lines(text)).is_none(), "{text}");
        }
    }

    #[test]
    fn unquoted_leaves_a_plain_path_and_decodes_escapes() {
        assert_eq!(unquoted("src/a.rs"), "src/a.rs");
        assert_eq!(unquoted("\"tab\\there\""), "tab\there");
        assert_eq!(unquoted("\"\\303\\251t\\303\\251\""), "été");
    }
}
