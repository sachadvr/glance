use std::path::{Path, PathBuf};

use crate::domain::{sort_by_directory, Commit, FileDiff, Status};
use crate::input::{parse_log, parse_patch};

use super::command::{lines, Git, GitError, GitResult};

// Git's well-known empty tree: what a root commit's changes are counted from.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
const MAX_UNTRACKED: usize = 200;
const MAX_FALLBACK_COMMITS: &str = "-n20";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// Uncommitted work, against HEAD.
    Worktree,
    /// The whole branch, from where it left its main line.
    Mainline,
    /// A commit of the branch onwards.
    Commit(String),
}

/// The repository glance was opened in, with what the header shows.
pub struct Repo {
    pub git: Git,
    pub name: String,
    pub branch: String,
    pub mainline: Option<String>,
    /// Where glance runs, from the root ("src/" in a subdirectory, empty at the root): what git's printed paths are relative to.
    pub prefix: String,
}

impl Repo {
    pub fn discover() -> GitResult<Repo> {
        Repo::discover_from(&Git::here())
    }

    /// The repository holding `directory`, wherever glance runs from: the tests open throwaway ones.
    pub fn open(directory: impl Into<PathBuf>) -> GitResult<Repo> {
        Repo::discover_from(&Git::at(directory))
    }

    fn discover_from(start: &Git) -> GitResult<Repo> {
        let located = start.run(&["rev-parse", "--show-toplevel", "--show-prefix"])?;
        let mut located = located.lines();
        let root = located.next().unwrap_or_default().to_string();
        let prefix = located.next().unwrap_or_default().to_string();
        let git = Git::at(&root);
        let branch = git.run(&["rev-parse", "--abbrev-ref", "HEAD"])?.trim().to_string();
        let name = Path::new(&root).file_name().map_or_else(|| root.clone(), |name| name.to_string_lossy().into_owned());
        let mainline = find_mainline(&git);

        Ok(Repo { git, name, branch, mainline, prefix })
    }

    fn revision(&self, base: &Base) -> GitResult<String> {
        match base {
            Base::Worktree => Ok("HEAD".to_string()),
            Base::Mainline => {
                let mainline = self.mainline.as_deref().ok_or(GitError::NoMainline)?;
                Ok(self.git.run(&["merge-base", mainline, "HEAD"])?.trim().to_string())
            }
            Base::Commit(hash) => Ok(self.git.run(&["rev-parse", "--verify", "--quiet", &format!("{hash}^")]).map_or_else(|_| EMPTY_TREE.to_string(), |parent| parent.trim().to_string())),
        }
    }

    pub fn files(&self, base: &Base) -> GitResult<Vec<FileDiff>> {
        let revision = self.revision(base)?;
        let mut files = parse_patch(&lines(&self.git.run(&["diff", "--no-color", "--no-ext-diff", "-M", &revision])?));
        let untracked = self.git.run(&["ls-files", "--others", "--exclude-standard", "-z"])?;
        for path in untracked.split('\0').filter(|path| !path.is_empty()).take(MAX_UNTRACKED) {
            // --no-index exits 1 when the files differ, which they always do here.
            let output = self.git.command(&["diff", "--no-color", "--no-ext-diff", "--no-index", "--", "/dev/null", path]).output().map_err(|error| GitError::Failed(error.to_string()))?;
            for mut file in parse_patch(&lines(&String::from_utf8_lossy(&output.stdout))) {
                file.status = Status::Untracked;
                file.path = path.to_string();
                files.push(file);
            }
        }
        sort_by_directory(&mut files);

        Ok(files)
    }

    /// The uncommitted work of the files a `git status` listed, with their patches. A listed file git has no patch
    /// for stays as listed; the paths stay as the status printed them.
    pub fn listed_files(&self, listed: &[FileDiff]) -> GitResult<Vec<FileDiff>> {
        Ok(with_patches(&self.prefix, listed, &self.files(&Base::Worktree)?))
    }

    pub fn commits(&self) -> GitResult<Vec<Commit>> {
        let range = self.mainline.as_ref().map(|mainline| format!("{mainline}..HEAD"));
        let mut args = vec!["log", "--no-color", "--no-ext-diff", "-p", "-M", "--date=relative", "--format=medium"];
        match &range {
            Some(range) => args.push(range),
            None => args.push(MAX_FALLBACK_COMMITS),
        }

        Ok(parse_log(&lines(&self.git.run(&args)?)))
    }
}

// "../README.md" printed in "src/" -> "README.md": a path git printed relative to `prefix`, from the root.
// `None` when it climbs above the root.
fn from_root(prefix: &str, printed: &str) -> Option<String> {
    let mut segments: Vec<&str> = prefix.split('/').filter(|segment| !segment.is_empty()).collect();
    for segment in printed.split('/') {
        match segment {
            ".." => {
                segments.pop()?;
            }
            "." | "" => {}
            _ => segments.push(segment),
        }
    }
    let mut path = segments.join("/");
    // An untracked directory is listed whole ("notes/"): its slash tells it from a file.
    if printed.ends_with('/') && !path.is_empty() {
        path.push('/');
    }
    Some(path)
}

// Each listed file takes the patch of the uncommitted file it names, or of every file under the directory it names.
// `status.relativePaths=false` prints paths from the root: a path not found from `prefix` is tried as such.
fn with_patches(prefix: &str, listed: &[FileDiff], uncommitted: &[FileDiff]) -> Vec<FileDiff> {
    let mut files = Vec::new();
    for entry in listed {
        let found = [from_root(prefix, &entry.path), Some(entry.path.clone())].into_iter().flatten().find_map(|root_path| {
            let named: Vec<FileDiff> = uncommitted.iter().filter(|file| names(&root_path, &file.path)).map(|file| printed_as(entry, &root_path, file)).collect();
            (!named.is_empty()).then_some(named)
        });
        match found {
            Some(named) => files.extend(named),
            None => files.push(entry.clone()),
        }
    }
    files
}

// "src/a.rs" names itself; "notes/" names every file under it.
fn names(listed: &str, path: &str) -> bool {
    path == listed || (listed.ends_with('/') && path.starts_with(listed))
}

// The uncommitted file under the path the status printed: a directory's file keeps the rest of its own path.
fn printed_as(entry: &FileDiff, root_path: &str, file: &FileDiff) -> FileDiff {
    let mut printed = file.clone();
    // "./" is the whole directory git ran in: its files read from there.
    let directory = entry.path.strip_prefix("./").unwrap_or(&entry.path);
    printed.path = format!("{directory}{}", &file.path[root_path.len()..]);
    if entry.old_path.is_some() {
        printed.old_path = entry.old_path.clone();
    }
    printed
}

fn find_mainline(git: &Git) -> Option<String> {
    if let Ok(head) = git.run(&["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"]) {
        return Some(head.trim().to_string()).filter(|head| !head.is_empty());
    }

    ["origin/master", "origin/main", "master", "main"].into_iter().find(|candidate| git.run(&["rev-parse", "--verify", "--quiet", candidate]).is_ok()).map(String::from)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process;

    use super::*;
    use crate::domain::Revision;

    // A throwaway repository: "main" with a root commit, "feature" one commit ahead, and an untracked file.
    struct TempRepo {
        root: PathBuf,
    }

    impl TempRepo {
        fn new(name: &str) -> TempRepo {
            let root = std::env::temp_dir().join(format!("glance-{}-{name}", process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let repo = TempRepo { root };
            repo.git(&["init", "--quiet", "--initial-branch=main"]);
            repo.commit("README.md", "hello\n", "Start");
            repo.git(&["checkout", "--quiet", "-b", "feature"]);
            repo.commit("src/a.rs", "fn a() {}\n", "Add a");
            fs::write(repo.root.join("notes.txt"), "draft\n").unwrap();
            repo
        }

        // The user's own config (signing, hooks) must not leak into the test.
        fn git(&self, args: &[&str]) -> String {
            let output = process::Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null"])
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }

        fn commit(&self, path: &str, content: &str, subject: &str) {
            let file = self.root.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, content).unwrap();
            self.git(&["add", path]);
            self.git(&["commit", "--quiet", "-m", subject]);
        }

        fn open(&self) -> Repo {
            Repo::discover_from(&Git::at(&self.root)).unwrap()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn paths(files: &[FileDiff]) -> Vec<(&str, Status)> {
        files.iter().map(|file| (file.path.as_str(), file.status)).collect()
    }

    #[test]
    fn discover_reads_the_branch_and_finds_a_local_mainline_without_a_remote() {
        let temp = TempRepo::new("discover");
        let repo = temp.open();

        assert_eq!((repo.branch.as_str(), repo.mainline.as_deref()), ("feature", Some("main")));
        assert!(repo.name.starts_with("glance-"));
    }

    #[test]
    fn files_against_the_mainline_hold_the_branch_and_the_untracked_files() {
        let temp = TempRepo::new("mainline");
        let repo = temp.open();

        assert_eq!(paths(&repo.files(&Base::Mainline).unwrap()), [("notes.txt", Status::Untracked), ("src/a.rs", Status::Added)]);
        assert_eq!(paths(&repo.files(&Base::Worktree).unwrap()), [("notes.txt", Status::Untracked)]);
    }

    #[test]
    fn files_from_the_root_commit_are_counted_from_the_empty_tree() {
        let temp = TempRepo::new("root");
        let repo = temp.open();
        let root_commit = temp.git(&["rev-list", "--max-parents=0", "HEAD"]);

        let files = repo.files(&Base::Commit(root_commit)).unwrap();

        assert_eq!(paths(&files), [("README.md", Status::Added), ("notes.txt", Status::Untracked), ("src/a.rs", Status::Added)]);
    }

    #[test]
    fn files_against_the_mainline_fail_without_one() {
        let temp = TempRepo::new("no-mainline");
        let mut repo = temp.open();
        repo.mainline = None;

        assert_eq!(repo.files(&Base::Mainline).unwrap_err(), GitError::NoMainline);
    }

    #[test]
    fn commits_list_the_branch_with_its_files() {
        let temp = TempRepo::new("commits");
        let repo = temp.open();

        let commits = repo.commits().unwrap();

        assert_eq!(commits.len(), 1);
        assert_eq!((commits[0].subject.as_str(), commits[0].author.as_str()), ("Add a", "Test"));
        assert_eq!(commits[0].files.as_deref().map(paths), Some(vec![("src/a.rs", Status::Added)]));
    }

    #[test]
    fn commits_fall_back_to_the_latest_without_a_mainline() {
        let temp = TempRepo::new("fallback");
        let mut repo = temp.open();
        repo.mainline = None;

        let subjects: Vec<String> = repo.commits().unwrap().into_iter().map(|commit| commit.subject).collect();

        assert_eq!(subjects, ["Add a", "Start"]);
    }

    // `git status` run in "src/", as git would pipe it to its pager, read back by the pager.
    fn status_in_src(temp: &TempRepo, pathspec: &[&str]) -> Vec<FileDiff> {
        let output = process::Command::new("git")
            .arg("-C")
            .arg(temp.root.join("src"))
            .args(["-c", "status.relativePaths=true", "-c", "color.status=false", "status", "--long", "--untracked-files=normal"])
            .args(pathspec)
            .output()
            .unwrap();
        let Some(crate::input::Document::Status(files)) = crate::input::read(&String::from_utf8_lossy(&output.stdout)) else { panic!("a status") };
        files
    }

    #[test]
    fn listed_files_take_the_patches_of_what_a_status_printed_where_git_ran() {
        let temp = TempRepo::new("status");
        fs::write(temp.root.join("README.md"), "changed\n").unwrap();
        fs::create_dir_all(temp.root.join("src/new")).unwrap();
        fs::write(temp.root.join("src/new/b.rs"), "fn b() {}\n").unwrap();
        let repo = Repo::discover_from(&Git::at(temp.root.join("src"))).unwrap();

        let mut everything = repo.listed_files(&status_in_src(&temp, &[])).unwrap();
        let mut here = repo.listed_files(&status_in_src(&temp, &["--", "."])).unwrap();
        sort_by_directory(&mut everything);

        assert_eq!(repo.prefix, "src/");
        assert_eq!(paths(&everything), [("../README.md", Status::Modified), ("../notes.txt", Status::Untracked), ("new/b.rs", Status::Untracked)]);
        assert!(everything.iter().all(|file| file.stats.changed() > 0));
        sort_by_directory(&mut here);
        assert_eq!(paths(&here), [("new/b.rs", Status::Untracked)]);
    }

    #[test]
    fn from_root_resolves_a_printed_path_against_the_prefix() {
        assert_eq!(from_root("src/", "a.rs").as_deref(), Some("src/a.rs"));
        assert_eq!(from_root("src/app/", "../../README.md").as_deref(), Some("README.md"));
        assert_eq!(from_root("", "./notes/").as_deref(), Some("notes/"));
        assert_eq!(from_root("src/", "../"), Some(String::new()));
        assert_eq!(from_root("src/", "../../outside"), None);
    }

    #[test]
    fn with_patches_keeps_what_git_has_no_patch_for_as_listed() {
        let mut changed = FileDiff::new("src/a.rs");
        changed.lines = vec!["@@ -1 +1 @@".to_string()];
        let mut renamed = FileDiff::new("src/new.rs");
        renamed.status = Status::Renamed;
        renamed.old_path = Some("src/old.rs".to_string());
        let mut listed_rename = FileDiff::new("new.rs");
        listed_rename.old_path = Some("old.rs".to_string());

        let files = with_patches("src/", &[FileDiff::new("a.rs"), listed_rename, FileDiff::new("gone.rs")], &[changed, renamed, FileDiff::new("unlisted.rs")]);

        let read: Vec<(&str, Option<&str>, usize)> = files.iter().map(|file| (file.path.as_str(), file.old_path.as_deref(), file.lines.len())).collect();
        assert_eq!(read, [("a.rs", None, 1), ("new.rs", Some("old.rs"), 0), ("gone.rs", None, 0)]);
        assert_eq!(files[1].status, Status::Renamed);
    }

    #[test]
    fn with_patches_spreads_an_untracked_directory_into_its_files() {
        let uncommitted = ["src/sub/n.rs", "src/sub/m.rs", "README.md"].map(FileDiff::new);

        let files = with_patches("src/", &[FileDiff::new("./")], &uncommitted);
        let nested = with_patches("", &[FileDiff::new("src/sub/")], &uncommitted);

        assert_eq!(files.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), ["sub/n.rs", "sub/m.rs"]);
        assert_eq!(nested.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), ["src/sub/n.rs", "src/sub/m.rs"]);
    }

    #[test]
    fn content_reads_a_file_at_a_revision_or_in_the_working_tree_from_any_subdirectory() {
        let temp = TempRepo::new("content");
        let head = Revision::Hash(temp.git(&["rev-parse", "HEAD"]));
        fs::write(temp.root.join("src/a.rs"), [0x89, b'P', b'N', b'G', 0xff]).unwrap();
        let from_subdirectory = Git::at(temp.root.join("src"));

        assert_eq!(from_subdirectory.content(Some(&head), "src/a.rs").unwrap(), b"fn a() {}\n");
        assert_eq!(from_subdirectory.content(None, "src/a.rs").unwrap(), [0x89, b'P', b'N', b'G', 0xff]);
        assert!(from_subdirectory.content(None, "missing.png").is_err());
        assert!(from_subdirectory.content(Some(&head), "notes.txt").is_err());
    }

    #[test]
    fn open_finds_the_repository_of_a_subdirectory() {
        let temp = TempRepo::new("open");
        assert_eq!(Repo::open(temp.root.join("src")).unwrap().branch, "feature");
    }

    #[test]
    fn git_loads_a_commit_its_subject_and_a_stash() {
        let temp = TempRepo::new("show");
        let repo = temp.open();
        let head = temp.git(&["rev-parse", "HEAD"]);

        assert_eq!(paths(&repo.git.files_of(&Revision::Hash(head.clone())).unwrap()), [("src/a.rs", Status::Added)]);
        assert_eq!(paths(&repo.git.files_of(&Revision::Branch("main".to_string())).unwrap()), [("README.md", Status::Added)]);
        assert_eq!(repo.git.subject(&head).unwrap(), "Add a");

        fs::write(temp.root.join("README.md"), "changed\n").unwrap();
        temp.git(&["stash", "push", "--quiet", "--include-untracked"]);
        assert_eq!(paths(&repo.git.files_of(&Revision::Stash("stash@{0}".to_string())).unwrap()), [("README.md", Status::Modified), ("notes.txt", Status::Added)]);
    }
}
