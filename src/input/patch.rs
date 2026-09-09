use crate::domain::{sort_by_directory, FileDiff, Status};

use super::Document;

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    if !lines.iter().any(|line| file_start_path(line).is_some()) {
        return None;
    }
    let mut files = parse_patch(lines);
    sort_by_directory(&mut files);

    Some(Document::Diff(files))
}

pub(crate) fn file_start_path(line: &str) -> Option<String> {
    if let Some(sides) = line.strip_prefix("diff --git ") {
        // "a/x b/x": the new side follows the last " b/"; a custom prefix is corrected by the "+++" line.
        return Some(sides.rfind(" b/").map_or(sides, |at| &sides[at + 3..]).to_string());
    }

    line.strip_prefix("diff --cc ").or_else(|| line.strip_prefix("diff --combined ")).map(str::to_string)
}

// "b/src/a.php" -> "src/a.php": git's side prefix is one letter and a slash, whatever diff.mnemonicPrefix picks.
fn without_side_prefix(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b'/' {
        return &path[2..];
    }

    path
}

pub fn parse_patch(lines: &[&str]) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut is_in_hunk = false;
    for line in lines {
        if let Some(path) = file_start_path(line) {
            files.push(FileDiff::new(path));
            is_in_hunk = false;
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if line.starts_with("@@") {
            is_in_hunk = true;
            file.lines.push(line.to_string());
            continue;
        }
        if is_in_hunk {
            match line.as_bytes().first() {
                Some(b'+') => file.stats.insertions += 1,
                Some(b'-') => file.stats.deletions += 1,
                _ => {}
            }
            file.lines.push(line.to_string());
            continue;
        }
        if line.starts_with("new file mode") {
            file.status = Status::Added;
        } else if line.starts_with("deleted file mode") {
            file.status = Status::Deleted;
        } else if let Some(from) = line.strip_prefix("rename from ") {
            file.old_path = Some(from.to_string());
        } else if let Some(to) = line.strip_prefix("rename to ") {
            file.status = Status::Renamed;
            file.path = to.to_string();
        } else if let Some(path) = line.strip_prefix("+++ ").filter(|path| *path != "/dev/null") {
            file.path = without_side_prefix(path).to_string();
        } else if line.starts_with("Binary files") {
            file.lines.push(line.to_string());
        }
    }

    files
}

#[cfg(test)]
pub(super) const DIFF: &str = "diff --git a/src/a.php b/src/a.php
index 1..2 100644
--- a/src/a.php
+++ b/src/a.php
@@ -1,3 +1,3 @@
 keep
--- removed line that looks like a header
+added
diff --git a/new.txt b/new.txt
new file mode 100644
--- /dev/null
+++ b/new.txt
@@ -0,0 +1 @@
+hello
diff --git a/old.txt b/renamed.txt
similarity index 100%
rename from old.txt
rename to renamed.txt
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ChangeStats;

    #[test]
    fn parse_patch_reads_status_counts_and_paths() {
        let lines: Vec<&str> = DIFF.lines().collect();
        let files = parse_patch(&lines);

        assert_eq!(files.len(), 3);
        assert_eq!((files[0].path.as_str(), files[0].status, files[0].stats), ("src/a.php", Status::Modified, ChangeStats { insertions: 1, deletions: 1 }));
        assert_eq!(files[0].lines.len(), 4);
        assert_eq!((files[1].path.as_str(), files[1].status, files[1].stats.insertions), ("new.txt", Status::Added, 1));
        assert_eq!((files[2].path.as_str(), files[2].status, files[2].old_path.as_deref()), ("renamed.txt", Status::Renamed, Some("old.txt")));
    }

    #[test]
    fn parse_patch_takes_the_path_from_a_mnemonic_prefix() {
        let files = parse_patch(&["diff --git i/x.rs w/x.rs", "--- i/x.rs", "+++ w/x.rs", "@@ -1 +1 @@", "-a", "+b"]);
        assert_eq!(files[0].path, "x.rs");
    }

    #[test]
    fn parse_patch_reads_deleted_binary_and_combined_files() {
        let files = parse_patch(&[
            "diff --git a/gone.txt b/gone.txt",
            "deleted file mode 100644",
            "--- a/gone.txt",
            "+++ /dev/null",
            "diff --git a/logo.png b/logo.png",
            "Binary files a/logo.png and b/logo.png differ",
            "diff --cc merged.rs",
            "@@@ -1,1 -1,1 +1,1 @@@",
        ]);

        let read: Vec<(&str, Status, usize)> = files.iter().map(|file| (file.path.as_str(), file.status, file.lines.len())).collect();
        assert_eq!(read, [("gone.txt", Status::Deleted, 0), ("logo.png", Status::Modified, 1), ("merged.rs", Status::Modified, 1)]);
    }

    #[test]
    fn parse_patch_ignores_what_comes_before_the_first_file() {
        assert!(parse_patch(&["@@ -1 +1 @@", "+orphan"]).is_empty());
    }
}
