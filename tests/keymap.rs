//! No key may be bound twice among the layers a screen is driven by: the first layer would silently win.

use std::rc::Rc;

use glance::i18n::FRENCH;
use glance::app::keymap::{self, char, key, Binding, Chord};
use glance::app::session::{Session, Source};
use glance::app::view::View;
use glance::app::views::{BasePickerView, BlameView, CommitFilesView, CommitsView, DiffView, FilesView};
use glance::domain::{Commit, FileDiff, Revision};
use glance::theme::ThemeSelection;
use ratatui::crossterm::event::KeyCode;

fn session() -> Session {
    Session::new(Source::Pager { command: "git log".to_string() })
}

fn files() -> Rc<[FileDiff]> {
    Rc::from(vec![FileDiff::new("src/a.rs"), FileDiff::new("src/b.rs")])
}

/// `App::root_bindings` is private: this mirrors the keys it adds at a tab's root, in its richest form (the repository's
/// files and commits tabs, plus the blame's own tab key), so a view key colliding with any of them is caught.
fn root_layer(roots: &[&dyn View], session: &Session) -> Vec<Chord> {
    let mut chords = vec![key(KeyCode::Esc), key(KeyCode::Tab)];
    chords.extend(roots.iter().filter_map(|view| view.tab_title(session, &FRENCH)).map(|title| char(title.key)));
    chords.extend(['1', '2', '3', 'r', 'F'].map(char));
    chords
}

fn chords(bindings: &[Binding]) -> Vec<Chord> {
    bindings.iter().flat_map(|binding| binding.chords.iter().copied()).collect()
}

fn duplicates(chords: &[Chord]) -> Vec<Chord> {
    let mut seen: Vec<Chord> = Vec::new();
    let mut repeated = Vec::new();
    for chord in chords {
        if seen.contains(chord) && !repeated.contains(chord) {
            repeated.push(*chord);
        }
        seen.push(*chord);
    }
    repeated
}

#[test]
fn no_screen_binds_a_key_twice_across_its_layers() {
    let session = session();
    let theme = ThemeSelection::default().current();
    let mut commit = Commit::new(Revision::Hash("abcdef123".into()), "Fix");
    commit.files = Some(files());
    let files_view = FilesView::default();
    let commits_view = CommitsView::default();
    let blame_view = BlameView::default();
    let roots: [&dyn View; 3] = [&files_view, &commits_view, &blame_view];
    let root_chords = root_layer(&roots, &session);
    let pushed: Vec<(&str, Box<dyn View>)> = vec![
        ("base picker", Box::new(BasePickerView::default())),
        ("commit files", Box::new(CommitFilesView::new(commit))),
        ("diff", Box::new(DiffView::new(files(), 0, theme))),
    ];

    for (name, view) in roots.iter().map(|view| ("root", *view)).chain(pushed.iter().map(|(name, view)| (*name, view.as_ref()))) {
        let mut layers = chords(&view.keymap(&FRENCH));
        if name == "root" {
            layers.extend(root_chords.iter().copied());
        }
        layers.extend(chords(&keymap::global(&FRENCH)));

        let label = view.tab_title(&session, &FRENCH).map_or(name.to_string(), |title| title.label.to_string());
        assert_eq!(duplicates(&layers), Vec::<Chord>::new(), "{label} binds a key twice");
    }
}

#[test]
fn the_root_layer_mirror_itself_has_no_duplicate() {
    let session = session();
    let files_view = FilesView::default();
    let commits_view = CommitsView::default();
    let blame_view = BlameView::default();

    let roots = root_layer(&[&files_view, &commits_view, &blame_view], &session);

    assert_eq!(duplicates(&roots), Vec::<Chord>::new());
    assert_eq!(duplicates(&chords(&keymap::search())), Vec::<Chord>::new());
}

#[test]
fn every_screen_names_at_least_one_key_and_global_keys_are_named() {
    let theme = ThemeSelection::default().current();
    let views: Vec<Box<dyn View>> = vec![
        Box::new(FilesView::default()),
        Box::new(CommitsView::default()),
        Box::new(BlameView::default()),
        Box::new(BasePickerView::default()),
        Box::new(DiffView::new(files(), 1, theme)),
    ];

    for view in &views {
        assert!(!keymap::hints(&view.keymap(&FRENCH)).is_empty());
    }
    assert_eq!(keymap::hints(&keymap::global(&FRENCH)), "/ chercher · y copier · t thème · q quitter");
}
