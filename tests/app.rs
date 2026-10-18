//! The app driven as a user would: keys, clicks and the screen they leave, against the behaviour of the
//! original glance (src/app.rs + ui.rs of the first version).

mod support;

use std::rc::Rc;

use glance::i18n::FRENCH;
use glance::app::command::Command;
use glance::app::App;
use glance::domain::{BlameLine, Commit, FileDiff, Revision};
use glance::input::Document;
use glance::theme::{ThemeSelection, THEMES};
use ratatui::crossterm::event::KeyCode;
use support::*;

const DIFF_HINTS: [&str; 5] = ["j/k défiler", "espace/b page", "n/p fichier", "w retour ligne", "← retour"];
const GLOBAL_HINTS: [&str; 4] = ["/ chercher", "y copier", "t thème", "q quitter"];

fn assert_hints(footer: &str, hints: &[&str]) {
    for hint in hints {
        assert!(footer.contains(hint), "footer {footer:?} misses {hint:?}");
    }
}

fn assert_no_hints(footer: &str, hints: &[&str]) {
    for hint in hints {
        assert!(!footer.contains(hint), "footer {footer:?} shows {hint:?}");
    }
}

fn wide_footer(app: &mut App) -> String {
    render(app, 200, 20).footer()
}

fn open_gamma(app: &mut App) {
    keys(app, "jj");
    key(app, KeyCode::Enter);
}

// ================================================================ tab content and footers

#[test]
fn files_tab_lists_the_files_under_their_directories_with_the_tests_last() {
    let mut app = app(&diff());

    let screen = frame(&mut app);

    assert!(screen.header().starts_with(" glance"), "{}", screen.header());
    assert!(screen.header().contains("git diff"));
    assert!(screen.header().contains(&format!("[t] {}", THEMES[0].name)));
    assert!(screen.tab_row().contains("[f] Fichiers 4"), "{}", screen.tab_row());
    let row = |needle: &str| screen.rows_containing(needle).first().copied().unwrap_or_else(|| panic!("{needle} not shown:\n{}", screen.text()));
    assert!(row(" src/") < row("alpha.rs"));
    assert!(row("alpha.rs") < row("beta.rs") && row("beta.rs") < row("gamma.rs"));
    assert!(row("gamma.rs") < row("Tests · 1 fichier") && row("Tests · 1 fichier") < row("alpha_test.rs"));
    assert!(screen.selected().is_some_and(|line| line.contains("alpha.rs")), "the cursor starts on the first file");
}

#[test]
fn files_tab_footer_names_its_keys_and_only_its_keys() {
    let mut app = app(&diff());

    let footer = wide_footer(&mut app);

    assert_hints(&footer, &["clic/⏎ diff"]);
    assert_hints(&footer, &GLOBAL_HINTS);
    // A single tab has nothing to switch to; a pager has no base, no reload, no fetch.
    assert_no_hints(&footer, &["onglet", "n/p fichier", "← retour", "base", "rafraîchir", "fetch", "déplier"]);
}

#[test]
fn commits_tab_shows_each_commit_and_its_footer() {
    let mut app = app(LOG);

    let screen = frame(&mut app);
    let footer = wide_footer(&mut app);

    assert!(screen.header().contains("git log"));
    assert!(screen.tab_row().contains("[c] Commits 2"), "{}", screen.tab_row());
    assert!(screen.tab_row().contains("[s] fichiers ▸"));
    assert!(screen.contains("1111111aa Fix the feed") && screen.contains("2222222bb Add the parser"), "{}", screen.text());
    assert!(screen.contains("Grace H · 2 days ago · 2 fichiers"), "{}", screen.text());
    assert_hints(&footer, &["clic/⏎ fichiers du commit", "s déplier"]);
    assert_hints(&footer, &GLOBAL_HINTS);
    assert_no_hints(&footer, &["← retour", "onglet"]);
}

#[test]
fn commit_files_screen_shows_the_commit_and_its_footer() {
    let mut app = app(LOG);
    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    let footer = wide_footer(&mut app);

    assert!(screen.contains("← 1111111aa Fix the feed"), "{}", screen.text());
    assert!(screen.contains("Grace H · 2 days ago"));
    assert!(screen.contains("src/feed.rs") && screen.contains("src/parser.rs"));
    assert_hints(&footer, &["clic/⏎ diff", "← retour"]);
    assert_hints(&footer, &GLOBAL_HINTS);
    assert_no_hints(&footer, &["déplier", "n/p fichier"]);
}

#[test]
fn diff_screen_shows_the_file_heading_and_its_footer() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    let footer = wide_footer(&mut app);

    let heading = screen.line(3);
    assert!(heading.starts_with(" ← M src/alpha.rs"), "{heading}");
    assert!(heading.contains("◀ préc  1/4  suiv ▶"), "{heading}");
    assert!(screen.contains("new_alpha();") && screen.contains("old_alpha();"));
    assert_hints(&footer, &DIFF_HINTS);
    assert_hints(&footer, &["/ chercher", "t thème", "q quitter"]);
    assert!(footer.contains("Y/y copier") || footer.contains("y/Y copier"), "both copies named: {footer}");
    assert_no_hints(&footer, &["clic/⏎"]);
}

#[test]
fn blame_tab_shows_who_wrote_each_line_and_its_footer() {
    let mut app = blame_app();

    let screen = frame(&mut app);
    let footer = wide_footer(&mut app);

    assert!(screen.header().contains("git blame  src/feed.php"), "{}", screen.header());
    assert!(screen.tab_row().contains("[b] Lignes 3"), "{}", screen.tab_row());
    assert!(screen.contains("7bfd4f45") && screen.contains("Linus") && screen.contains("2024-08-21"), "{}", screen.text());
    assert!(screen.contains("<?php") && screen.contains("return $feed;"));
    assert!(screen.contains("pas encore commitée"));
    assert_hints(&footer, &["clic/⏎ commit de la ligne"]);
    assert_hints(&footer, &GLOBAL_HINTS);
}

#[test]
fn a_stash_list_is_a_stash_tab() {
    let mut app = app(STASH);

    let screen = frame(&mut app);

    assert!(screen.header().contains("git stash"), "{}", screen.header());
    assert!(screen.tab_row().contains("[c] Stash 2"), "{}", screen.tab_row());
    assert!(screen.contains("stash@{0} WIP on main: 238d615 Fix the feed"), "{}", screen.text());
}

#[test]
fn a_branch_list_is_a_branches_tab_that_marks_the_current_branch() {
    let mut app = app(BRANCHES);

    let screen = frame(&mut app);

    assert!(screen.header().contains("git branch"), "{}", screen.header());
    assert!(screen.tab_row().contains("[c] Branches 3"), "{}", screen.tab_row());
    assert!(screen.rows_containing("  * main Fix the feed").len() == 1 && screen.contains("0123456"), "{}", screen.text());
    assert!(screen.contains("feature/search [origin/feature/search: ahead 1] Add the search") && screen.contains("89abcde"), "{}", screen.text());
    assert!(screen.contains("été Corrige l'écran"), "{}", screen.text());
    assert!(!screen.contains("* feature") && !screen.contains("* été"), "{}", screen.text());
}

#[test]
fn a_single_branch_stays_a_list_of_branches() {
    let mut app = app("* main\n");

    let screen = frame(&mut app);

    assert!(screen.tab_row().contains("[c] Branches 1") && screen.contains("* main"), "{}", screen.text());
    assert_hints(&screen.footer(), &["fichiers du commit"]);
}

#[test]
fn a_branch_is_searched_and_copied_by_its_name() {
    let mut app = app(BRANCHES);

    search(&mut app, "écran");

    assert_eq!(yank(&mut app).as_deref(), Some("été"));
}

#[test]
fn a_detached_head_opens_on_the_files_of_its_commit() {
    let mut app = app(DETACHED_BRANCHES);
    assert!(frame(&mut app).contains("* HEAD (HEAD detached at 0123456)"));

    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    assert!(screen.contains("← HEAD (HEAD detached at 0123456)"), "{}", screen.text());
    assert_hints(&screen.footer(), &["← retour"]);
}

#[test]
fn a_status_lists_its_files_under_their_directories_with_the_tests_last() {
    for text in [STATUS, STATUS_SHORT] {
        let mut app = app(text);

        let screen = frame(&mut app);

        assert!(screen.header().contains("git status"), "{}", screen.header());
        assert!(screen.tab_row().contains("[f] Fichiers 4"), "{}", screen.tab_row());
        let order = ["glance-fixture/app/", "M feed.rs", "D parser.rs", "glance-fixture/docs/", "R new.md", "Tests · 1 fichier", "? feed_test.rs"].map(|needle| screen.rows_containing(needle).first().copied());
        assert!(order.iter().all(Option::is_some) && order.is_sorted(), "{order:?}\n{}", screen.text());
    }
}

#[test]
fn a_status_file_git_has_no_patch_for_still_opens() {
    let mut app = app(STATUS);

    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    assert!(screen.contains("feed.rs"), "{}", screen.text());
    assert_hints(&wide_footer(&mut app), &["n/p fichier", "← retour"]);
}

#[test]
fn a_reflog_shows_its_selectors_and_a_oneline_log_has_no_byline() {
    let mut reflog = app(REFLOG);
    let mut oneline = app(ONELINE);

    let reflog_screen = frame(&mut reflog);
    let oneline_screen = frame(&mut oneline);

    assert!(reflog_screen.tab_row().contains("[c] Commits 3"));
    assert!(reflog_screen.contains("HEAD@{1}") && reflog_screen.contains("commit: Init hotfix"), "{}", reflog_screen.text());
    assert!(oneline_screen.contains("0123456 First") && oneline_screen.contains("fedcba9 Third"), "{}", oneline_screen.text());
    // Neither files nor stats are known yet: unknown, not zero.
    assert!(!oneline_screen.contains("fichier "), "{}", oneline_screen.text());
}

// ================================================================ empty states

#[test]
fn an_empty_diff_says_so_and_ignores_open_search_and_copy() {
    let mut app = App::for_document(Document::Diff(Vec::new()), None, ThemeSelection::default(), &FRENCH);

    key(&mut app, KeyCode::Enter);
    keys(&mut app, "jGk");
    assert_eq!(yank(&mut app), None);
    let screen = frame(&mut app);

    assert!(screen.contains("Aucune modification."), "{}", screen.text());
    assert!(screen.tab_row().contains("[f] Fichiers 0"));
    assert!(!app.should_quit());
    search(&mut app, "x");
    assert_eq!(screen_footer(&mut app), "aucun résultat");
}

#[test]
fn an_empty_log_says_so_and_ignores_open_and_unfold() {
    let mut app = App::for_document(Document::Log(Vec::new()), None, ThemeSelection::default(), &FRENCH);

    key(&mut app, KeyCode::Enter);
    keys(&mut app, "s");
    let screen = frame(&mut app);

    assert!(screen.contains("Aucun commit sur la branche."), "{}", screen.text());
    assert!(screen.tab_row().contains("[c] Commits 0"));
    assert_eq!(yank(&mut app), None);
    assert!(!app.should_quit());
}

#[test]
fn an_empty_blame_draws_and_ignores_open() {
    let mut app = App::for_document(Document::Blame(Vec::new()), None, ThemeSelection::default(), &FRENCH);

    key(&mut app, KeyCode::Enter);
    let screen = frame(&mut app);

    assert!(screen.tab_row().contains("[b] Lignes 0"));
    assert_eq!(yank(&mut app), None);
    assert!(!app.should_quit());
}

#[test]
fn a_merge_commit_of_a_log_with_patches_opens_on_an_empty_file_list() {
    let log = "commit 1111111aaaa\nMerge: 2222222 3333333\nAuthor: Lea <lea@example.com>\n\n    Merge branch 'x'\n\ncommit 3333333cccc\nAuthor: Lea <lea@example.com>\n\n    Add\n\ndiff --git a/a b/a\n@@ -1 +1 @@\n+x\n";
    let mut app = app(log);

    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    keys(&mut app, "jG");
    let screen = frame(&mut app);

    assert!(screen.contains("← 1111111aa Merge branch 'x'"), "{}", screen.text());
    assert_hints(&screen.footer(), &["← retour"]);
    key(&mut app, KeyCode::Esc);
    assert!(screen_text(&mut app).contains("[s] fichiers"), "back on the commits");
}

fn screen_footer(app: &mut App) -> String {
    frame(app).footer()
}

fn screen_text(app: &mut App) -> String {
    frame(app).text()
}

// ================================================================ navigation

#[test]
fn back_from_a_diff_browsed_with_n_leaves_the_cursor_on_the_last_file_viewed() {
    for back in [KeyCode::Esc, KeyCode::Left, KeyCode::Char('h'), KeyCode::Backspace] {
        let mut app = app(&diff());
        key(&mut app, KeyCode::Enter);
        keys(&mut app, "nn");
        assert!(frame(&mut app).line(3).contains("src/gamma.rs"), "{back:?}");

        key(&mut app, back);

        let screen = frame(&mut app);
        assert!(screen.tab_row().contains("[f] Fichiers"), "{back:?}: {}", screen.text());
        assert!(screen.selected().is_some_and(|line| line.contains("gamma.rs")), "{back:?}: {}", screen.text());
        assert_eq!(yank(&mut app).as_deref(), Some("src/gamma.rs"), "{back:?}");
        assert!(!app.should_quit());
    }
}

#[test]
fn n_and_p_stop_at_the_first_and_last_files() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);

    keys(&mut app, "p[");
    assert!(frame(&mut app).line(3).contains("1/4"));
    keys(&mut app, "nnnnnnn]]");
    assert!(frame(&mut app).line(3).contains("4/4"));
    assert!(frame(&mut app).line(3).contains("tests/alpha_test.rs"));
    keys(&mut app, "p");
    assert!(frame(&mut app).line(3).contains("3/4 "));
    key(&mut app, KeyCode::Esc);
    assert_eq!(yank(&mut app).as_deref(), Some("src/gamma.rs"));
}

#[test]
fn enter_in_a_diff_scrolls_rather_than_reopening() {
    let mut app = app(&diff());
    open_gamma(&mut app);

    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    assert!(screen.line(3).contains("src/gamma.rs"));
    assert!(screen.line(5).contains("gamma line 01"), "{}", screen.text());
}

#[test]
fn commit_to_files_to_diff_and_back_keeps_each_cursor() {
    let mut app = app(LOG);

    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    let files = frame(&mut app);
    assert!(files.contains("← 2222222bb Add the parser"), "{}", files.text());
    assert!(files.contains("Tests · 1 fichier"));

    key(&mut app, KeyCode::Enter);
    keys(&mut app, "n");
    let diff = frame(&mut app);
    assert!(diff.line(3).contains("tests/parser_test.rs") && diff.line(3).contains("2/2"), "{}", diff.text());

    key(&mut app, KeyCode::Esc);
    let files = frame(&mut app);
    assert!(files.selected().is_some_and(|line| line.contains("parser_test.rs")), "{}", files.text());
    assert_eq!(yank(&mut app).as_deref(), Some("2222222bb"), "y in a commit copies its hash");

    key(&mut app, KeyCode::Esc);
    let commits = frame(&mut app);
    assert!(commits.selected().is_some_and(|line| line.contains("2222222bb")), "{}", commits.text());
    assert!(!app.should_quit());

    key(&mut app, KeyCode::Esc);
    assert!(app.should_quit());
}

#[test]
fn a_single_commit_log_opens_straight_on_its_files() {
    let mut app = app(SHOW);

    let screen = frame(&mut app);

    assert!(screen.contains("← 1111111aa Fix the feed"), "{}", screen.text());
    assert!(screen.selected().is_some_and(|line| line.contains("feed.rs")));
    assert_hints(&screen.footer(), &["← retour"]);
    key(&mut app, KeyCode::Esc);
    assert!(!app.should_quit(), "back goes to the commit list first, as before");
    assert!(screen_text(&mut app).contains("[c] Commits 1"));
    key(&mut app, KeyCode::Esc);
    assert!(app.should_quit());
}

#[test]
fn a_single_commit_without_its_patch_reports_gits_error_and_stays_on_the_list() {
    let mut app = app("0123456 Lonely\n");

    let screen = frame(&mut app);

    assert!(screen.contains("0123456 Lonely"), "{}", screen.text());
    assert!(!screen.contains("← 0123456"));
    let footer = screen.footer();
    assert!(!footer.is_empty() && !footer.contains("clic/⏎"), "git's error, not the hints: {footer:?}");
}

#[test]
fn opening_a_commit_git_cannot_show_reports_the_error_and_stays() {
    for log in [ONELINE, LOG_NO_PATCH, STASH] {
        let mut app = app(log);
        keys(&mut app, "j");

        key(&mut app, KeyCode::Enter);

        let screen = frame(&mut app);
        assert!(!screen.contains(" ← "), "{log}: {}", screen.text());
        assert!(!screen.footer().is_empty() && !screen.footer().contains("clic/⏎"), "{log}: {:?}", screen.footer());
        keys(&mut app, "j");
        assert!(screen_footer(&mut app).contains("clic/⏎"), "the next key clears the error");
    }
}

#[test]
fn unfolding_a_log_git_cannot_show_reports_the_error_and_folds_back() {
    let mut app = app(ONELINE);

    keys(&mut app, "s");
    let screen = frame(&mut app);

    assert!(screen.tab_row().contains("[s] fichiers ▸"), "{}", screen.tab_row());
    assert!(!screen.footer().contains("clic/⏎"), "{:?}", screen.footer());
}

#[test]
fn unfolding_a_log_with_patches_lists_each_commits_files() {
    let mut app = app(LOG);

    keys(&mut app, "s");
    let screen = frame(&mut app);

    assert!(screen.tab_row().contains("[s] fichiers ▾"), "{}", screen.tab_row());
    assert!(screen.rows_containing("feed.rs").len() == 1 && screen.rows_containing("parser_test.rs").len() == 1, "{}", screen.text());
    keys(&mut app, "s");
    assert!(!screen_text(&mut app).contains("feed.rs"));
}

#[test]
fn an_uncommitted_blame_line_does_not_open() {
    let mut app = blame_app();

    keys(&mut app, "G");
    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    assert_eq!(screen.footer(), "ligne pas encore commitée");
    assert!(screen.tab_row().contains("[b] Lignes"));
    assert_eq!(yank(&mut app).as_deref(), Some("00000000"));
}

#[test]
fn a_committed_blame_line_git_cannot_show_reports_gits_error() {
    let mut app = blame_app();

    key(&mut app, KeyCode::Enter);

    let screen = frame(&mut app);
    let footer = screen.footer();
    assert!(!footer.is_empty() && footer != "ligne pas encore commitée" && !footer.contains("clic/⏎"), "{footer:?}");
    assert!(screen.tab_row().contains("[b] Lignes"));
    assert!(!app.should_quit());
}

// ================================================================ search

#[test]
fn the_prompt_echoes_what_is_typed_and_backspace_erases() {
    let mut app = app(&diff());

    keys(&mut app, "/alx");
    key(&mut app, KeyCode::Backspace);
    keys(&mut app, "pha");

    assert_eq!(screen_footer(&mut app), "/alpha");
    keys(&mut app, "q");
    assert!(!app.should_quit(), "q is typed into the prompt");
    assert_eq!(screen_footer(&mut app), "/alphaq");
}

#[test]
fn enter_jumps_to_the_first_match_and_n_and_shift_n_wrap_around() {
    let mut app = app(&diff());
    keys(&mut app, "j");

    search(&mut app, "ALPHA");
    assert_eq!(screen_footer(&mut app), "/ALPHA · n/N · esc effacer");
    assert_eq!(yank(&mut app).as_deref(), Some("tests/alpha_test.rs"), "from beta, the next match going down");

    keys(&mut app, "n");
    assert_eq!(yank(&mut app).as_deref(), Some("src/alpha.rs"), "n wraps to the top");
    keys(&mut app, "N");
    assert_eq!(yank(&mut app).as_deref(), Some("tests/alpha_test.rs"), "N wraps to the bottom");
    keys(&mut app, "N");
    assert_eq!(yank(&mut app).as_deref(), Some("src/alpha.rs"));
}

#[test]
fn confirming_a_search_finds_the_item_under_the_cursor_too() {
    let mut app = app(&diff());

    search(&mut app, "alpha");

    assert_eq!(yank(&mut app).as_deref(), Some("src/alpha.rs"));
}

#[test]
fn a_search_without_a_match_says_aucun_resultat_and_keeps_the_cursor() {
    let mut app = app(&diff());
    keys(&mut app, "j");

    search(&mut app, "zzz");

    assert_eq!(screen_footer(&mut app), "aucun résultat");
    assert_eq!(yank(&mut app).as_deref(), Some("src/beta.rs"));
}

#[test]
fn esc_clears_the_search_and_a_second_esc_quits() {
    let mut app = app(&diff());
    search(&mut app, "gamma");

    key(&mut app, KeyCode::Esc);

    assert!(!app.should_quit(), "the first Esc clears the search");
    assert!(screen_footer(&mut app).contains("clic/⏎ diff"));
    keys(&mut app, "n");
    assert_eq!(yank(&mut app).as_deref(), Some("src/gamma.rs"), "n no longer jumps");
    key(&mut app, KeyCode::Esc);
    assert!(app.should_quit());
}

#[test]
fn esc_in_the_prompt_abandons_it_and_an_empty_query_clears_the_search() {
    let mut app = app(&diff());
    search(&mut app, "beta");

    keys(&mut app, "/gam");
    key(&mut app, KeyCode::Esc);
    assert_eq!(screen_footer(&mut app), "/beta · n/N · esc effacer", "the confirmed search stays");
    assert!(!app.should_quit());

    keys(&mut app, "/");
    key(&mut app, KeyCode::Enter);
    assert!(screen_footer(&mut app).contains("clic/⏎ diff"));
}

#[test]
fn opening_a_diff_ends_the_search_and_n_steps_files_again() {
    let mut app = app(&diff());
    search(&mut app, "beta");

    key(&mut app, KeyCode::Enter);
    let footer = wide_footer(&mut app);

    assert!(!footer.contains("n/N"), "{footer}");
    assert_hints(&footer, &DIFF_HINTS);
    keys(&mut app, "n");
    assert!(frame(&mut app).line(3).contains("src/gamma.rs"));
    key(&mut app, KeyCode::Esc);
    assert!(screen_footer(&mut app).contains("clic/⏎ diff"), "the list's search is gone too");
}

#[test]
fn a_search_in_a_diff_scrolls_to_the_match_and_n_takes_over_from_next_file() {
    let mut app = app(&diff());
    open_gamma(&mut app);

    search(&mut app, "line 20");
    let screen = frame(&mut app);
    assert!(screen.line(5).contains("gamma line 20"), "{}", screen.text());

    keys(&mut app, "n");
    assert!(screen_text(&mut app).contains("src/gamma.rs"), "n jumps within the diff, not to the next file");
    key(&mut app, KeyCode::Esc);
    assert!(frame(&mut app).line(3).contains("src/gamma.rs"), "Esc clears the search before going back");
    key(&mut app, KeyCode::Esc);
    assert_eq!(yank(&mut app).as_deref(), Some("src/gamma.rs"));
}

#[test]
fn a_search_near_the_end_of_a_diff_keeps_the_last_page_full() {
    let mut app = app(&diff());
    open_gamma(&mut app);
    frame(&mut app);

    search(&mut app, "line 59");

    let screen = frame(&mut app);
    assert!(screen.line(18).contains("gamma line 60"), "{}", screen.text());
    assert!(screen.contains("gamma line 59"));
}

#[test]
fn a_search_matches_commits_by_author_and_blame_lines_by_code() {
    let mut log = app(LOG);
    search(&mut log, "lea");
    assert_eq!(yank(&mut log).as_deref(), Some("2222222bb"));

    let mut blame = blame_app();
    search(&mut blame, "RETURN");
    assert_eq!(yank(&mut blame).as_deref(), Some("238d6155"));
}

#[test]
fn changing_file_in_a_diff_ends_the_search_as_before() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);
    search(&mut app, "alpha");

    keys(&mut app, "]");

    let screen = frame(&mut app);
    assert!(screen.line(3).contains("src/beta.rs"));
    assert!(!screen.footer().contains("n/N"), "{:?}", screen.footer());
}

#[test]
fn n_wraps_around_a_diff_even_past_its_last_page() {
    let mut app = app(&diff());
    open_gamma(&mut app);
    frame(&mut app);
    search(&mut app, "5");
    assert!(frame(&mut app).line(5).contains("gamma line 05"));

    let mut wrapped = false;
    for _ in 0..20 {
        keys(&mut app, "n");
        wrapped |= frame(&mut app).line(5).contains("gamma line 05");
    }

    assert!(wrapped, "n should come back to the first match");
}

// ================================================================ copy

#[test]
fn y_copies_what_the_screen_is_about_and_says_so() {
    let mut app = app(&diff());

    keys(&mut app, "y");
    assert_eq!(screen_footer(&mut app), "copié : src/alpha.rs");
    assert_eq!(app.take_clipboard().as_deref(), Some("src/alpha.rs"));
    assert_eq!(app.take_clipboard(), None, "the clipboard is handed over once");

    keys(&mut app, "j");
    assert!(screen_footer(&mut app).contains("clic/⏎ diff"), "the next key clears the notice");
    keys(&mut app, "Y");
    assert_eq!(app.take_clipboard(), None, "Y copies a whole diff only");

    key(&mut app, KeyCode::Enter);
    assert_eq!(yank(&mut app).as_deref(), Some("src/beta.rs"), "y in a diff copies its path");
}

#[test]
fn shift_y_copies_the_whole_diff_and_the_notice_shows_its_start_on_one_line() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);

    keys(&mut app, "Y");

    let copied = app.take_clipboard().expect("the diff");
    assert!(copied.starts_with("@@ -1,3 +1,3 @@\n fn alpha() {\n-    old_alpha();\n+    new_alpha();"), "{copied}");
    assert!(copied.ends_with("\n }"), "{copied}");
    let shown: String = copied.replace('\n', " ").chars().take(40).collect();
    assert_eq!(screen_footer(&mut app), format!("copié : {shown}…"));
}

#[test]
fn y_copies_hashes_in_logs_and_blames() {
    let mut log = app(LOG);
    assert_eq!(yank(&mut log).as_deref(), Some("1111111aa"));
    keys(&mut log, "j");
    assert_eq!(yank(&mut log).as_deref(), Some("2222222bb"));

    let mut stash = app(STASH);
    keys(&mut stash, "j");
    assert_eq!(yank(&mut stash).as_deref(), Some("stash@{1}"));

    let mut blame = blame_app();
    keys(&mut blame, "j");
    assert_eq!(yank(&mut blame).as_deref(), Some("238d6155"));
    assert_eq!(screen_footer(&mut blame), "copié : 238d6155");
}

// ================================================================ wrap

fn wrapped_rows(app: &mut App) -> usize {
    frame(app).rows_containing("wrapword").len()
}

#[test]
fn w_wraps_long_lines_and_the_choice_outlives_the_diff() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);
    assert_eq!(wrapped_rows(&mut app), 1, "cut at the edge by default");

    keys(&mut app, "w");
    assert!(wrapped_rows(&mut app) >= 2);

    keys(&mut app, "n");
    assert!(wrapped_rows(&mut app) >= 2, "the next file wraps too");
    key(&mut app, KeyCode::Esc);
    keys(&mut app, "k");
    key(&mut app, KeyCode::Enter);
    assert!(frame(&mut app).line(3).contains("src/alpha.rs"));
    assert!(wrapped_rows(&mut app) >= 2, "a diff opened again wraps too");

    keys(&mut app, "w");
    assert_eq!(wrapped_rows(&mut app), 1);
}

// ================================================================ scrolling

#[test]
fn scrolling_a_diff_stops_with_its_last_row_at_the_bottom() {
    let mut app = app(&diff());
    open_gamma(&mut app);
    frame(&mut app);

    keys(&mut app, "G");
    let at_end = frame(&mut app).text();
    assert!(frame(&mut app).line(18).contains("gamma line 60"), "{at_end}");

    keys(&mut app, &"j".repeat(100));
    key(&mut app, KeyCode::PageDown);
    keys(&mut app, "   ");
    ctrl(&mut app, 'd');
    key(&mut app, KeyCode::End);
    for _ in 0..30 {
        wheel_down(&mut app);
    }
    assert_eq!(frame(&mut app).text(), at_end);

    keys(&mut app, "k");
    assert!(frame(&mut app).line(18).contains("gamma line 59"));
}

#[test]
fn scrolling_a_diff_up_stops_at_its_first_row() {
    let mut app = app(&diff());
    open_gamma(&mut app);
    let top = frame(&mut app).text();

    keys(&mut app, "kkkb");
    key(&mut app, KeyCode::PageUp);
    ctrl(&mut app, 'u');
    wheel_up(&mut app);

    assert_eq!(frame(&mut app).text(), top);
    keys(&mut app, &"j".repeat(40));
    key(&mut app, KeyCode::Home);
    assert_eq!(frame(&mut app).text(), top);
    keys(&mut app, &"j".repeat(40));
    keys(&mut app, "g");
    assert_eq!(frame(&mut app).text(), top);
}

#[test]
fn page_keys_move_by_the_rows_the_diff_shows() {
    let mut app = app(&diff());
    open_gamma(&mut app);
    frame(&mut app);

    keys(&mut app, " ");
    // 14 diff rows fit under the heading at 80x20: the hunk row and lines 01 to 13 scroll away.
    assert!(frame(&mut app).line(5).contains("gamma line 14"), "{}", frame(&mut app).text());
    keys(&mut app, "b");
    assert!(frame(&mut app).line(5).contains("@@"));
    ctrl(&mut app, 'd');
    assert!(frame(&mut app).line(5).contains("gamma line 07"));
}

#[test]
fn the_wheel_moves_a_diff_by_three_rows_and_a_list_by_one() {
    let mut diff_app = app(&diff());
    open_gamma(&mut diff_app);
    frame(&mut diff_app);
    wheel_down(&mut diff_app);
    assert!(frame(&mut diff_app).line(5).contains("gamma line 03"));

    let mut list = app(&diff());
    wheel_down(&mut list);
    assert_eq!(yank(&mut list).as_deref(), Some("src/beta.rs"));
    for _ in 0..10 {
        wheel_down(&mut list);
    }
    assert_eq!(yank(&mut list).as_deref(), Some("tests/alpha_test.rs"), "the cursor stops on the last file");
    wheel_up(&mut list);
    assert_eq!(yank(&mut list).as_deref(), Some("src/gamma.rs"));
}

#[test]
fn the_wheel_waits_for_the_search_prompt_to_close() {
    let mut app = app(&diff());
    keys(&mut app, "/be");

    wheel_down(&mut app);
    key(&mut app, KeyCode::Esc);

    assert_eq!(yank(&mut app).as_deref(), Some("src/alpha.rs"));
}

#[test]
fn the_wheel_clears_an_error_like_a_key_does() {
    let mut app = app(&diff());
    search(&mut app, "zzz");
    assert_eq!(screen_footer(&mut app), "aucun résultat");

    wheel_down(&mut app);

    // The search stays confirmed after a miss, as in the old app: its line replaces the error.
    assert_eq!(screen_footer(&mut app), "/zzz · n/N · esc effacer");
}

// ================================================================ mouse

#[test]
fn a_click_on_a_file_row_opens_its_diff() {
    let mut app = app(&diff());
    let at = frame(&mut app).find("beta.rs").expect("beta listed");

    click(&mut app, at);

    let screen = frame(&mut app);
    assert!(screen.line(3).contains("src/beta.rs") && screen.line(3).contains("2/4"), "{}", screen.text());
}

#[test]
fn a_click_on_the_back_arrow_returns_with_the_cursor_on_the_file() {
    let mut app = app(&diff());
    let gamma = frame(&mut app).find("gamma.rs").expect("gamma listed");
    click(&mut app, gamma);
    let back = frame(&mut app).find("←").expect("a back arrow");
    assert_eq!(back.1, 3, "the arrow heads the diff");

    click(&mut app, back);

    let screen = frame(&mut app);
    assert!(screen.contains("Tests · 1 fichier"), "{}", screen.text());
    assert!(screen.selected().is_some_and(|line| line.contains("gamma.rs")));
}

#[test]
fn clicks_on_next_and_previous_step_through_the_files() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);
    let next = frame(&mut app).find("suiv ▶").expect("next");

    click(&mut app, next);
    assert!(frame(&mut app).line(3).contains("src/beta.rs"));
    let previous = frame(&mut app).find("◀ préc").expect("previous");
    click(&mut app, previous);
    assert!(frame(&mut app).line(3).contains("src/alpha.rs"));
}

#[test]
fn a_click_on_any_line_of_a_commit_opens_it_and_its_arrow_goes_back() {
    let mut app = app(LOG);
    let byline = frame(&mut app).find("Lea · 3 days ago").expect("second commit's byline");

    click(&mut app, byline);
    let screen_after = frame(&mut app);
    assert!(screen_after.contains("← 2222222bb Add the parser"), "{}", screen_after.text());

    let feed = screen_after.find("parser_test.rs").expect("listed");
    click(&mut app, feed);
    assert!(frame(&mut app).line(3).contains("tests/parser_test.rs"));
    let back = frame(&mut app).find("←").expect("arrow");
    click(&mut app, back);
    let back = frame(&mut app).find("←").expect("the commit's arrow");
    click(&mut app, back);

    let commits = frame(&mut app);
    assert!(commits.selected().is_some_and(|line| line.contains("2222222bb")), "{}", commits.text());
}

#[test]
fn a_click_on_the_unfold_tool_toggles_the_files() {
    let mut app = app(LOG);
    let tool = frame(&mut app).find("[s] fichiers").expect("tool");

    click(&mut app, tool);

    assert!(frame(&mut app).tab_row().contains("[s] fichiers ▾"));
}

#[test]
fn a_click_while_the_prompt_is_open_does_nothing() {
    let mut app = app(&diff());
    let beta = frame(&mut app).find("beta.rs").expect("listed");
    keys(&mut app, "/be");

    click(&mut app, beta);

    assert_eq!(screen_footer(&mut app), "/be");
    assert!(frame(&mut app).contains("Tests · 1 fichier"), "still on the list");
}

#[test]
fn a_click_outside_any_target_does_nothing() {
    let mut app = app(&diff());
    frame(&mut app);

    click(&mut app, (79, 19));
    click(&mut app, (200, 200));

    assert!(frame(&mut app).contains("Tests · 1 fichier"));
    assert!(!app.should_quit());
}

#[test]
fn hovered_reports_the_clickable_area_under_the_mouse() {
    let mut app = app(&diff());
    let beta = frame(&mut app).find("beta.rs").expect("listed");

    hover(&mut app, beta);
    frame(&mut app);
    let hit = app.hovered().expect("a row is clickable");
    assert_eq!(hit.command, Command::OpenAt(1));
    assert!(hit.area.y == beta.1 && hit.area.x == 0 && hit.area.width == 80, "{hit:?}");

    let theme = frame(&mut app).find("[t]").expect("theme switch");
    hover(&mut app, theme);
    assert_eq!(app.hovered().map(|hit| hit.command.clone()), Some(Command::CycleTheme));

    hover(&mut app, (40, 19));
    assert_eq!(app.hovered(), None, "the footer is not clickable");
    let directory = frame(&mut app).find(" src/").expect("heading");
    hover(&mut app, directory);
    assert_eq!(app.hovered(), None, "a directory heading is not clickable");
}

#[test]
fn hovered_follows_the_screen_drawn_last() {
    let mut app = app(&diff());
    let first_row = frame(&mut app).find("alpha.rs").expect("listed");
    hover(&mut app, first_row);
    frame(&mut app);
    assert!(matches!(app.hovered().map(|hit| &hit.command), Some(Command::OpenAt(0))));

    key(&mut app, KeyCode::Enter);
    frame(&mut app);

    assert!(!matches!(app.hovered().map(|hit| &hit.command), Some(Command::OpenAt(_))), "{:?}", app.hovered());
}

// ================================================================ quitting

#[test]
fn esc_and_q_quit_from_every_root() {
    for text in [diff(), LOG.to_string(), BLAME.to_string(), STASH.to_string()] {
        for quit in [KeyCode::Esc, KeyCode::Char('q')] {
            let mut app = app(&text);
            assert!(!app.should_quit());
            key(&mut app, quit);
            assert!(app.should_quit(), "{quit:?} on {text}");
        }
    }
}

#[test]
fn q_quits_from_inside_a_diff_but_esc_goes_back() {
    let mut app = app(&diff());
    key(&mut app, KeyCode::Enter);

    key(&mut app, KeyCode::Esc);
    assert!(!app.should_quit());
    key(&mut app, KeyCode::Enter);
    keys(&mut app, "q");
    assert!(app.should_quit());
}

#[test]
fn ctrl_c_quits_from_inside_the_search_prompt_and_from_a_diff() {
    let mut prompt = app(&diff());
    keys(&mut prompt, "/alp");
    ctrl(&mut prompt, 'c');
    assert!(prompt.should_quit());

    let mut diff_app = app(&diff());
    key(&mut diff_app, KeyCode::Enter);
    ctrl(&mut diff_app, 'c');
    assert!(diff_app.should_quit());
}

#[test]
fn other_control_keys_in_the_prompt_are_not_typed() {
    let mut app = app(&diff());
    keys(&mut app, "/al");

    ctrl(&mut app, 'u');
    ctrl(&mut app, 'w');
    key(&mut app, KeyCode::Tab);

    assert_eq!(screen_footer(&mut app), "/al");
    assert!(!app.should_quit());
}

// ================================================================ tiny terminals

fn every_screen() -> Vec<(&'static str, App)> {
    let mut screens = Vec::new();
    screens.push(("files", app(&diff())));
    let mut diff_screen = app(&diff());
    key(&mut diff_screen, KeyCode::Enter);
    keys(&mut diff_screen, "w");
    search(&mut diff_screen, "wrapword");
    screens.push(("diff wrapped and searched", diff_screen));
    let mut gamma = app(&diff());
    open_gamma(&mut gamma);
    keys(&mut gamma, "G");
    screens.push(("diff at its end", gamma));
    screens.push(("commits", app(LOG)));
    let mut unfolded = app(LOG);
    keys(&mut unfolded, "s");
    screens.push(("commits unfolded", unfolded));
    let mut commit = app(LOG);
    key(&mut commit, KeyCode::Enter);
    screens.push(("commit files", commit));
    let mut commit_diff = app(LOG);
    key(&mut commit_diff, KeyCode::Enter);
    key(&mut commit_diff, KeyCode::Enter);
    screens.push(("commit diff", commit_diff));
    screens.push(("show", app(SHOW)));
    screens.push(("blame", blame_app()));
    screens.push(("stash", app(STASH)));
    let mut prompt = app(ONELINE);
    keys(&mut prompt, "/somequery");
    screens.push(("prompt", prompt));
    let mut error = app(ONELINE);
    key(&mut error, KeyCode::Enter);
    screens.push(("error", error));
    screens.push(("empty diff", App::for_document(Document::Diff(Vec::new()), None, ThemeSelection::default(), &FRENCH)));
    screens.push(("empty log", App::for_document(Document::Log(Vec::new()), None, ThemeSelection::default(), &FRENCH)));
    screens.push(("empty blame", App::for_document(Document::Blame(Vec::new()), None, ThemeSelection::default(), &FRENCH)));
    screens
}

#[test]
fn every_screen_draws_at_tiny_sizes_without_panicking() {
    for (name, mut app) in every_screen() {
        for (width, height) in [(1, 1), (10, 5), (2, 3), (5, 1), (1, 5), (80, 1), (80, 2), (1, 40), (12, 4), (0, 0), (3, 0), (0, 3)] {
            hover(&mut app, (width / 2, height / 2));
            let screen = render(&mut app, width, height);
            assert_eq!((screen.width, screen.height), (width, height), "{name}");
        }
    }
}

#[test]
fn every_screen_takes_keys_and_clicks_after_a_tiny_draw() {
    for (name, mut app) in every_screen() {
        render(&mut app, 3, 2);
        for code in [KeyCode::PageDown, KeyCode::End, KeyCode::PageUp, KeyCode::Down, KeyCode::Char('n'), KeyCode::Char('p')] {
            key(&mut app, code);
        }
        ctrl(&mut app, 'd');
        for x in 0..3 {
            for y in 0..2 {
                click(&mut app, (x, y));
                render(&mut app, 3, 2);
            }
        }
        // Back at a usable size, the screen is whole again.
        let screen = frame(&mut app);
        assert!(screen.header().contains("glance"), "{name}: {}", screen.text());
    }
}

#[test]
fn a_hover_far_outside_a_small_frame_is_harmless() {
    let mut app = app(&diff());
    frame(&mut app);
    hover(&mut app, (u16::MAX, u16::MAX));

    render(&mut app, 4, 4);

    assert_eq!(app.hovered(), None);
}

// ================================================================ documents built by hand

#[test]
fn a_log_whose_files_are_known_but_empty_and_a_blame_without_path_draw() {
    let mut commit = Commit::new(Revision::Hash("abcdef123".into()), "Nothing");
    commit.files = Some(Rc::from(Vec::<FileDiff>::new()));
    let mut log = App::for_document(Document::Log(vec![commit]), None, ThemeSelection::default(), &FRENCH);
    assert!(frame(&mut log).contains("← abcdef123 Nothing"));
    key(&mut log, KeyCode::Enter);
    assert!(frame(&mut log).contains("← abcdef123 Nothing"), "no file to open");

    let line = BlameLine { hash: "7bfd4f45".into(), author: "A very long author name indeed".into(), date: "2024-08-21".into(), number: 99999, code: "\tindented\tcode".into() };
    let mut blame = App::for_document(Document::Blame(vec![line]), None, ThemeSelection::default(), &FRENCH);
    let screen = frame(&mut blame);
    assert!(screen.header().contains("git blame"));
    assert!(screen.contains("indented"), "{}", screen.text());
}

#[test]
fn a_binary_and_a_rename_only_file_open_on_what_git_said() {
    let text = "diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\ndiff --git a/old.txt b/new.txt\nsimilarity index 100%\nrename from old.txt\nrename to new.txt\n";
    let mut app = app(text);

    key(&mut app, KeyCode::Enter);
    let binary = frame(&mut app);
    assert!(binary.contains("Binary files a/logo.png and b/logo.png differ"), "{}", binary.text());
    keys(&mut app, "jGk ");
    search(&mut app, "nothing");
    assert_eq!(screen_footer(&mut app), "aucun résultat");

    key(&mut app, KeyCode::Esc);
    keys(&mut app, "n");
    let rename = frame(&mut app);
    assert!(rename.line(3).contains("new.txt") && rename.line(3).contains("depuis old.txt"), "{}", rename.text());
    keys(&mut app, "jGY");
    assert_eq!(app.take_clipboard().as_deref(), Some(""));
}
