//! What glance says, in each language it speaks. A language is one `Strings` value in its own file, listed in
//! `LANGUAGES`: a struct literal, so a translation missing a text does not compile.

mod en;
mod fr;

use crate::domain::Status;
use crate::git::GitError;

pub use en::ENGLISH;
pub use fr::FRENCH;

/// Every language glance speaks; the first is the fallback.
pub const LANGUAGES: [&Strings; 2] = [&ENGLISH, &FRENCH];

/// The texts of one language. Texts with a number or a name in them are functions: each language puts them
/// (and agrees its plurals) its own way.
pub struct Strings {
    /// The ISO 639-1 code a locale starts with ("fr" in "fr_FR.UTF-8").
    pub code: &'static str,

    // Footer hints: what a key does.
    pub click_or_enter: &'static str,
    pub space_key: &'static str,
    pub open_diff: &'static str,
    pub open_commit_files: &'static str,
    pub open_line_commit: &'static str,
    pub choose: &'static str,
    pub cancel: &'static str,
    pub back: &'static str,
    pub search: &'static str,
    pub copy: &'static str,
    pub theme: &'static str,
    pub quit: &'static str,
    pub tab: &'static str,
    pub base: &'static str,
    pub reload: &'static str,
    pub fetch: &'static str,
    pub unfold: &'static str,
    pub scroll: &'static str,
    pub page: &'static str,
    pub file: &'static str,
    pub wrap_lines: &'static str,
    /// The line shown while a search is confirmed: "/query · n/N · esc to clear".
    pub search_status: fn(&str) -> String,

    // Header, tabs and the base row.
    pub files_tab: &'static str,
    pub commits_tab: &'static str,
    pub stash_tab: &'static str,
    pub branches_tab: &'static str,
    pub lines_tab: &'static str,
    pub commit_files_tool: &'static str,
    pub fetch_running: &'static str,
    pub fetch_done: &'static str,
    pub fetch_failed: &'static str,
    pub since: &'static str,
    pub worktree: &'static str,
    pub other_base: &'static str,
    pub since_commit: fn(&str) -> String,

    // Screens.
    pub no_changes: &'static str,
    pub no_commits: &'static str,
    pub pick_base_title: &'static str,
    pub not_committed_yet: &'static str,
    pub previous_file: &'static str,
    pub next_file: &'static str,
    /// In place of an image the file view could not read or decode.
    pub image_unavailable: &'static str,
    pub file_count: fn(usize) -> String,
    pub tests_section: fn(usize) -> String,
    pub more_files: fn(usize) -> String,
    pub status: fn(Status) -> &'static str,
    /// "renamed from old/path.rs".
    pub status_from: fn(Status, &str) -> String,

    // Messages under the screen.
    pub no_result: &'static str,
    pub no_commit_on_branch: &'static str,
    pub line_not_committed: &'static str,
    pub copied: fn(&str) -> String,
    pub no_mainline: &'static str,
}

impl Strings {
    pub fn git_error(&self, error: &GitError) -> String {
        match error {
            GitError::NoMainline => self.no_mainline.to_string(),
            GitError::Failed(message) => message.clone(),
        }
    }
}

/// The language of the first variable set among `GLANCE_LANG`, `LC_ALL`, `LC_MESSAGES` and `LANG`, as POSIX
/// ranks them; English when it is none glance speaks. The environment is passed in so the rule is testable.
pub fn detect(variable: impl Fn(&str) -> Option<String>) -> &'static Strings {
    let locale = ["GLANCE_LANG", "LC_ALL", "LC_MESSAGES", "LANG"].into_iter().filter_map(&variable).find(|value| !value.is_empty());
    locale.and_then(|locale| language(&locale)).unwrap_or(LANGUAGES[0])
}

// "fr_FR.UTF-8", "fr-CA", "fr" → French. "C" and "POSIX" name no language.
fn language(locale: &str) -> Option<&'static Strings> {
    let code = locale.split(['_', '-', '.', '@']).next()?.to_ascii_lowercase();
    LANGUAGES.into_iter().find(|strings| strings.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string())
    }

    #[test]
    fn detect_reads_the_language_of_the_locale() {
        assert_eq!(detect(environment(&[("LANG", "fr_FR.UTF-8")])).code, "fr");
        assert_eq!(detect(environment(&[("LANG", "en_US.UTF-8")])).code, "en");
    }

    #[test]
    fn detect_ranks_the_override_then_lc_all_then_lc_messages_then_lang() {
        assert_eq!(detect(environment(&[("GLANCE_LANG", "en"), ("LC_ALL", "fr_FR.UTF-8")])).code, "en");
        assert_eq!(detect(environment(&[("LC_ALL", "fr_FR.UTF-8"), ("LANG", "en_US.UTF-8")])).code, "fr");
        assert_eq!(detect(environment(&[("LC_MESSAGES", "fr"), ("LANG", "en_US.UTF-8")])).code, "fr");
    }

    #[test]
    fn detect_skips_empty_variables_and_falls_back_to_english() {
        assert_eq!(detect(environment(&[("LC_ALL", ""), ("LANG", "fr_BE.UTF-8")])).code, "fr");
        assert_eq!(detect(environment(&[("LANG", "C")])).code, "en");
        assert_eq!(detect(environment(&[("LANG", "de_DE.UTF-8")])).code, "en");
        assert_eq!(detect(environment(&[])).code, "en");
    }

    #[test]
    fn every_language_has_a_unique_code() {
        for (index, strings) in LANGUAGES.iter().enumerate() {
            assert!(LANGUAGES[..index].iter().all(|earlier| earlier.code != strings.code), "{} is listed twice", strings.code);
        }
    }

    #[test]
    fn plurals_agree_with_their_count() {
        assert_eq!(((ENGLISH.file_count)(1), (ENGLISH.file_count)(2)), ("1 file".to_string(), "2 files".to_string()));
        assert_eq!(((FRENCH.file_count)(1), (FRENCH.file_count)(2)), ("1 fichier".to_string(), "2 fichiers".to_string()));
    }

    #[test]
    fn a_git_failure_keeps_what_git_said() {
        assert_eq!(FRENCH.git_error(&GitError::Failed("fatal: bad revision".into())), "fatal: bad revision");
        assert_eq!(ENGLISH.git_error(&GitError::NoMainline), ENGLISH.no_mainline);
    }
}
