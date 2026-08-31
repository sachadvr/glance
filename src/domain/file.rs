use std::iter::Sum;
use std::ops::Add;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl Status {
    pub fn letter(self) -> char {
        match self {
            Status::Modified => 'M',
            Status::Added => 'A',
            Status::Deleted => 'D',
            Status::Renamed => 'R',
            Status::Untracked => '?',
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Modified => "modifié",
            Status::Added => "ajouté",
            Status::Deleted => "supprimé",
            Status::Renamed => "renommé",
            Status::Untracked => "non suivi",
        }
    }
}

/// Lines added and removed: per file, per commit, per list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeStats {
    pub insertions: usize,
    pub deletions: usize,
}

impl ChangeStats {
    pub fn changed(self) -> usize {
        self.insertions + self.deletions
    }
}

impl Add for ChangeStats {
    type Output = ChangeStats;

    fn add(self, other: ChangeStats) -> ChangeStats {
        ChangeStats { insertions: self.insertions + other.insertions, deletions: self.deletions + other.deletions }
    }
}

impl Sum for ChangeStats {
    fn sum<I: Iterator<Item = ChangeStats>>(iter: I) -> ChangeStats {
        iter.fold(ChangeStats::default(), Add::add)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: Status,
    pub stats: ChangeStats,
    /// The hunks as git printed them, headers of the file excluded.
    pub lines: Vec<String>,
}

const TEST_DIRECTORIES: [&str; 5] = ["tests", "test", "__tests__", "spec", "specs"];

impl FileDiff {
    pub fn new(path: impl Into<String>) -> Self {
        FileDiff { path: path.into(), old_path: None, status: Status::Modified, stats: ChangeStats::default(), lines: Vec::new() }
    }

    pub fn directory(&self) -> &str {
        self.path.rfind('/').map_or("", |slash| &self.path[..=slash])
    }

    pub fn name(&self) -> &str {
        self.path.rfind('/').map_or(self.path.as_str(), |slash| &self.path[slash + 1..])
    }

    // Test directories (tests/, __tests__/, spec/) or test-named files (FooTest.php, foo.test.ts, foo_test.go).
    pub fn is_test(&self) -> bool {
        let in_test_directory = self.directory().split('/').any(|segment| TEST_DIRECTORIES.contains(&segment));
        let stem = self.name().split('.').next().unwrap_or("");
        let is_test_named = stem.ends_with("Test") || stem.ends_with("_test") || self.name().contains(".test.") || self.name().contains(".spec.");

        in_test_directory || is_test_named
    }
}

// The code first, then the tests, each grouped under its directories: sorting by plain path would split a
// directory's files around a sibling directory ("a/x", "a/b/y", "a/z").
pub fn sort_by_directory(files: &mut [FileDiff]) {
    files.sort_by(|left, right| (left.is_test(), left.directory(), left.name()).cmp(&(right.is_test(), right.directory(), right.name())));
}

pub fn total_stats(files: &[FileDiff]) -> ChangeStats {
    files.iter().map(|file| file.stats).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_by_directory_keeps_a_directory_files_together_and_tests_last() {
        let mut files = ["tests/unit/ATest.php", "a/z.rs", "a/b/y.rs", "a/x.rs", "root.md"].map(FileDiff::new).to_vec();
        sort_by_directory(&mut files);
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, ["root.md", "a/x.rs", "a/z.rs", "a/b/y.rs", "tests/unit/ATest.php"]);
    }

    #[test]
    fn is_test_recognises_test_directories_and_test_names() {
        let is_test = |path: &str| FileDiff::new(path).is_test();
        assert!(is_test("tests/functional/Core/FooControllerTest.php"));
        assert!(is_test("src/components/__tests__/Card.ts"));
        assert!(is_test("src/Feed/WriterTest.php"));
        assert!(is_test("src/stores/cart.spec.ts"));
        assert!(is_test("pkg/feed_test.go"));
        assert!(!is_test("src/Core/Testing/Contest.php"));
        assert!(!is_test("docs/TESTING.md"));
    }

    #[test]
    fn directory_and_name_split_at_the_last_slash() {
        let file = FileDiff::new("src/Core/A.php");
        assert_eq!((file.directory(), file.name()), ("src/Core/", "A.php"));
        let root = FileDiff::new("README.md");
        assert_eq!((root.directory(), root.name()), ("", "README.md"));
    }

    #[test]
    fn total_stats_adds_every_file() {
        let mut first = FileDiff::new("a");
        first.stats = ChangeStats { insertions: 2, deletions: 1 };
        let mut second = FileDiff::new("b");
        second.stats = ChangeStats { insertions: 3, deletions: 0 };
        assert_eq!(total_stats(&[first, second]), ChangeStats { insertions: 5, deletions: 1 });
    }
}
