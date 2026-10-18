use crate::domain::{Commit, Revision};

use super::log::is_hash;
use super::Document;

// git refuses these in a ref name (`git check-ref-format`), on top of whitespace and control characters.
const FORBIDDEN_IN_REF: [char; 6] = ['~', '^', ':', '?', '*', '['];

// What a detached HEAD reads as between parentheses ("(HEAD detached at abc1234)", "(no branch, rebasing main)").
const DETACHED_STATES: [&str; 3] = ["HEAD detached at ", "HEAD detached from ", "no branch"];

pub(super) fn detect(lines: &[&str]) -> Option<Document> {
    parse_branch_list(lines).map(Document::Log)
}

fn is_ref_name(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('-')
        && !text.ends_with('/')
        && !text.contains("..")
        && !text.contains("@{")
        && !text.chars().any(|character| character.is_whitespace() || character.is_control() || character == '\\' || FORBIDDEN_IN_REF.contains(&character))
}

// "* main      abc1234 [ahead 1] Fix the feed": a `git branch` line, "*" the branch checked out, "+" one checked
// out in another worktree; -v adds the tip's short hash and subject, -a the remote branches.
fn branch_line(line: &str) -> Option<Commit> {
    let current = match line.get(..2)? {
        "* " => true,
        "  " | "+ " => false,
        _ => return None,
    };
    let rest = &line[2..];
    let (name, detached, details) = match rest.strip_prefix('(') {
        Some(inside) => {
            let (state, details) = inside.split_once(')')?;
            if !DETACHED_STATES.iter().any(|start| state.starts_with(start)) {
                return None;
            }
            // `git show HEAD` is what a detached HEAD opens on.
            ("HEAD", Some(format!("({state})")), details)
        }
        None => {
            let (name, details) = rest.split_once(' ').unwrap_or((rest, ""));
            (name, None, details)
        }
    };
    if !is_ref_name(name) {
        return None;
    }

    let details = details.trim_start();
    let (hash, subject) = match details.strip_prefix("-> ") {
        // "remotes/origin/HEAD -> origin/main": an alias, opened through its own name.
        Some(target) if is_ref_name(target) => ("", details),
        Some(_) => return None,
        None if details.is_empty() => ("", ""),
        None => {
            let (hash, subject) = details.split_once(' ').unwrap_or((details, ""));
            if !is_hash(hash) {
                return None;
            }
            (hash, subject)
        }
    };
    let subject = match detached {
        Some(state) if subject.is_empty() => state,
        Some(state) => format!("{state} {subject}"),
        None => subject.to_string(),
    };

    let mut branch = Commit::new(Revision::Branch(name.to_string()), subject);
    branch.when = hash.to_string();
    branch.current = current;
    Some(branch)
}

fn parse_branch_list(lines: &[&str]) -> Option<Vec<Commit>> {
    let branches = lines.iter().filter(|line| !line.trim().is_empty()).map(|line| branch_line(line)).collect::<Option<Vec<Commit>>>()?;

    (!branches.is_empty()).then_some(branches)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    fn summary(branches: &[Commit]) -> Vec<(&str, bool, &str, &str)> {
        branches.iter().map(|branch| (branch.revision.as_str(), branch.current, branch.when.as_str(), branch.subject.as_str())).collect()
    }

    #[test]
    fn parse_branch_list_reads_names_and_marks_the_current_branch() {
        let branches = parse_branch_list(&lines("  feature/search\n* main\n+ hotfix\n  deadbeef\n")).unwrap();

        assert_eq!(summary(&branches), [("feature/search", false, "", ""), ("main", true, "", ""), ("hotfix", false, "", ""), ("deadbeef", false, "", "")]);
        assert!(branches.iter().all(|branch| branch.is_branch() && branch.files.is_none()));
    }

    #[test]
    fn parse_branch_list_reads_the_tip_of_each_branch_with_verbose() {
        let verbose = "  feature 89abcde [origin/feature: ahead 1, behind 2] Add the flag\n* main    0123456 Fix the feed\n  empty   fedcba9 \n";

        let branches = parse_branch_list(&lines(verbose)).unwrap();

        assert_eq!(summary(&branches), [("feature", false, "89abcde", "[origin/feature: ahead 1, behind 2] Add the flag"), ("main", true, "0123456", "Fix the feed"), ("empty", false, "fedcba9", "")]);
    }

    #[test]
    fn parse_branch_list_reads_remote_branches_and_their_alias() {
        let all = "* main\n  remotes/origin/HEAD -> origin/main\n  remotes/origin/main\n";
        let verbose = "* main                0123456 Fix\n  remotes/origin/HEAD -> origin/main\n";

        assert_eq!(summary(&parse_branch_list(&lines(all)).unwrap())[1], ("remotes/origin/HEAD", false, "", "-> origin/main"));
        assert_eq!(summary(&parse_branch_list(&lines(verbose)).unwrap())[1], ("remotes/origin/HEAD", false, "", "-> origin/main"));
    }

    #[test]
    fn parse_branch_list_opens_a_detached_head_as_head() {
        let branches = parse_branch_list(&lines("* (HEAD detached at abc1234)\n  main\n")).unwrap();
        let verbose = parse_branch_list(&lines("* (no branch, rebasing main) abc1234 Fix the feed\n  main                       0123456 Start\n")).unwrap();

        assert_eq!(summary(&branches)[0], ("HEAD", true, "", "(HEAD detached at abc1234)"));
        assert_eq!(summary(&verbose)[0], ("HEAD", true, "abc1234", "(no branch, rebasing main) Fix the feed"));
    }

    #[test]
    fn parse_branch_list_reads_multibyte_names_and_subjects() {
        let branches = parse_branch_list(&lines("* fonctionnalité/été 0123456 Corrige l'écran 🚀\n")).unwrap();

        assert_eq!(summary(&branches), [("fonctionnalité/été", true, "0123456", "Corrige l'écran 🚀")]);
    }

    #[test]
    fn parse_branch_list_refuses_what_is_not_a_branch_list() {
        for text in [
            "0123456 First\n89abcde Second\n",
            "* 0123456 (HEAD -> main) First\n",
            " src/a.php | 3 +-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n",
            "Grace (2):\n      Fix the feed\n      Add the flag\n",
            "On branch main\nnothing to commit, working tree clean\n",
            " M src/a.rs\n?? notes.txt\n",
            "* main\nnot a branch\n",
            "* main\n  a:b\n",
            "* (detached somewhere)\n",
            "* main wrong subject\n",
            "",
        ] {
            assert!(parse_branch_list(&lines(text)).is_none(), "{text}");
        }
    }

    #[test]
    fn is_ref_name_follows_what_git_allows() {
        assert!(is_ref_name("feature/search") && is_ref_name("été") && is_ref_name("v1.2"));
        assert!(!is_ref_name("") && !is_ref_name("-x") && !is_ref_name("a..b") && !is_ref_name("a b") && !is_ref_name("a\tb") && !is_ref_name("a~1") && !is_ref_name("a\\b") && !is_ref_name("HEAD@{1}") && !is_ref_name("dir/"));
    }
}
