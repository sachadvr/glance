//! The interface speaks the language it was given, on every screen, and no other.

mod support;

use glance::app::App;
use glance::i18n::{Strings, ENGLISH, FRENCH, LANGUAGES};
use glance::theme::ThemeSelection;
use ratatui::crossterm::event::KeyCode;
use support::*;

fn app_in(text: &str, strings: &'static Strings) -> App {
    App::for_document(document(text), None, ThemeSelection::default(), strings)
}

// Words only the French interface uses: one of them on an English screen is a text left untranslated.
const FRENCH_WORDS: [&str; 12] = ["fichier", "chercher", "copier", "quitter", "onglet", "retour", "défiler", "Lignes", "Aucun", "commitée", "préc", "suiv"];

fn assert_english(screen: &Screen, what: &str) {
    for word in FRENCH_WORDS {
        assert!(!screen.contains(word), "{what} shows the French word {word:?}:\n{}", screen.text());
    }
}

#[test]
fn the_files_screen_and_its_diff_speak_english() {
    let mut app = app_in(&diff(), &ENGLISH);
    let files = frame(&mut app);
    assert!(files.tab_row().contains("[f] Files 4"), "{}", files.text());
    assert!(files.contains("Tests · 1 file"), "{}", files.text());
    assert!(files.footer().starts_with("click/⏎ diff"), "{}", files.footer());
    assert_english(&files, "the files screen");

    key(&mut app, KeyCode::Enter);
    let diff = frame(&mut app);
    assert!(diff.contains("next ▶") && diff.footer().contains("j/k scroll · space/b page · n/p file"), "{}", diff.text());
    assert_english(&diff, "a diff");
}

#[test]
fn the_commit_screens_speak_english() {
    let mut app = app_in(LOG, &ENGLISH);
    keys(&mut app, "s");
    let commits = frame(&mut app);
    assert!(commits.tab_row().contains("[c] Commits 2") && commits.contains("2 files"), "{}", commits.text());
    assert!(commits.footer().contains("click/⏎ commit files · s unfold"), "{}", commits.footer());
    assert_english(&commits, "the commit list");

    key(&mut app, KeyCode::Enter);
    assert_english(&frame(&mut app), "a commit's files");
}

#[test]
fn a_stash_list_and_a_blame_speak_english() {
    let stash = frame(&mut app_in(STASH, &ENGLISH));
    assert!(stash.tab_row().contains("[c] Stash 2"), "{}", stash.text());

    let mut blame = App::for_document(document(BLAME), Some("src/feed.php".to_string()), ThemeSelection::default(), &ENGLISH);
    let screen = frame(&mut blame);
    assert!(screen.tab_row().contains("[b] Lines 3") && screen.contains("not committed yet"), "{}", screen.text());
    assert_english(&screen, "a blame");

    keys(&mut blame, "G");
    key(&mut blame, KeyCode::Enter);
    assert_eq!(frame(&mut blame).footer(), "this line is not committed yet");
}

#[test]
fn messages_under_the_screen_speak_english() {
    let mut app = app_in(&diff(), &ENGLISH);

    search(&mut app, "zzz");
    assert_eq!(frame(&mut app).footer(), "no match");
    key(&mut app, KeyCode::Esc);

    search(&mut app, "alpha");
    assert_eq!(frame(&mut app).footer(), "/alpha · n/N · esc to clear");
    key(&mut app, KeyCode::Esc);

    keys(&mut app, "y");
    assert_eq!(frame(&mut app).footer(), "copied: src/alpha.rs");
}

#[test]
fn the_french_interface_stays_french() {
    let mut app = app_in(&diff(), &FRENCH);
    let files = frame(&mut app);
    assert!(files.tab_row().contains("[f] Fichiers 4") && files.footer().starts_with("clic/⏎ diff"), "{}", files.text());
}

#[test]
fn every_language_draws_every_screen() {
    for strings in LANGUAGES {
        for text in [diff(), LOG.to_string(), ONELINE.to_string(), REFLOG.to_string(), STASH.to_string(), BLAME.to_string()] {
            let mut app = app_in(&text, strings);
            render(&mut app, 80, 20);
            key(&mut app, KeyCode::Enter);
            render(&mut app, 80, 20);
            render(&mut app, 12, 4);
        }
    }
}
