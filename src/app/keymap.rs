//! Keys as data: what a key does and how the footer names it are declared once, side by side.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::command::{Command, Motion};

/// A key as bindings name it. Shift is part of the character ('G'), Control is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub code: KeyCode,
    pub control: bool,
}

pub const fn key(code: KeyCode) -> Chord {
    Chord { code, control: false }
}

pub const fn char(character: char) -> Chord {
    key(KeyCode::Char(character))
}

pub const fn ctrl(character: char) -> Chord {
    Chord { code: KeyCode::Char(character), control: true }
}

impl Chord {
    pub fn matches(self, event: &KeyEvent) -> bool {
        self.code == event.code && self.control == event.modifiers.contains(KeyModifiers::CONTROL)
    }
}

/// How the footer names a binding: "⏎ diff". Bindings sharing a label are named together ("f/c onglet").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub keys: String,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub chords: Vec<Chord>,
    pub command: Command,
    pub hint: Option<Hint>,
}

impl Binding {
    pub fn new(command: Command, chords: &[Chord]) -> Binding {
        Binding { chords: chords.to_vec(), command, hint: None }
    }

    pub fn hint(mut self, keys: impl Into<String>, label: &'static str) -> Binding {
        self.hint = Some(Hint { keys: keys.into(), label });
        self
    }
}

/// The first binding of the first layer that knows the key wins.
pub fn resolve<'a>(layers: impl IntoIterator<Item = &'a [Binding]>, event: &KeyEvent) -> Option<Command> {
    layers.into_iter().flatten().find(|binding| binding.chords.iter().any(|chord| chord.matches(event))).map(|binding| binding.command.clone())
}

/// The footer's hints, in binding order: "j/k défiler · espace/b page".
pub fn hints<'a>(bindings: impl IntoIterator<Item = &'a Binding>) -> String {
    let mut groups: Vec<(String, &str)> = Vec::new();
    for hint in bindings.into_iter().filter_map(|binding| binding.hint.as_ref()) {
        match groups.iter_mut().find(|(_, label)| *label == hint.label) {
            Some((keys, _)) => {
                keys.push('/');
                keys.push_str(&hint.keys);
            }
            None => groups.push((hint.keys.clone(), hint.label)),
        }
    }
    groups.iter().map(|(keys, label)| format!("{keys} {label}")).collect::<Vec<_>>().join(" · ")
}

/// Cursor keys every list shares.
pub fn list_motions() -> Vec<Binding> {
    vec![
        Binding::new(Command::Move(Motion::Down), &[key(KeyCode::Down), char('j')]),
        Binding::new(Command::Move(Motion::Up), &[key(KeyCode::Up), char('k')]),
        Binding::new(Command::Move(Motion::PageDown), &[key(KeyCode::PageDown)]),
        Binding::new(Command::Move(Motion::PageUp), &[key(KeyCode::PageUp)]),
        Binding::new(Command::Move(Motion::Top), &[char('g'), key(KeyCode::Home)]),
        Binding::new(Command::Move(Motion::Bottom), &[char('G'), key(KeyCode::End)]),
    ]
}

pub fn open(label: &'static str) -> Binding {
    Binding::new(Command::Open, &[key(KeyCode::Enter), key(KeyCode::Right), char('l')]).hint("clic/⏎", label)
}

pub fn back() -> Binding {
    Binding::new(Command::Back, &[key(KeyCode::Esc), key(KeyCode::Left), char('h'), key(KeyCode::Backspace)]).hint("←", "retour")
}

/// What works everywhere, after the screen's own keys.
pub fn global() -> Vec<Binding> {
    vec![
        Binding::new(Command::StartSearch, &[char('/')]).hint("/", "chercher"),
        Binding::new(Command::Yank, &[char('y')]).hint("y", "copier"),
        Binding::new(Command::CycleTheme, &[char('t')]).hint("t", "thème"),
        Binding::new(Command::Quit, &[char('q')]).hint("q", "quitter"),
    ]
}

/// While a search is confirmed, n/N step through its matches before any screen sees them.
pub fn search() -> Vec<Binding> {
    vec![
        Binding::new(Command::SearchNext, &[char('n')]),
        Binding::new(Command::SearchPrevious, &[char('N')]),
        Binding::new(Command::ClearSearch, &[key(KeyCode::Esc)]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_group_bindings_sharing_a_label() {
        let bindings = [
            Binding::new(Command::SwitchTab(0), &[char('f')]).hint("f", "onglet"),
            Binding::new(Command::Reload, &[char('r')]).hint("r", "rafraîchir"),
            Binding::new(Command::SwitchTab(1), &[char('c')]).hint("c", "onglet"),
            Binding::new(Command::Move(Motion::Down), &[char('j')]),
        ];
        assert_eq!(hints(&bindings), "f/c onglet · r rafraîchir");
    }

    #[test]
    fn resolve_takes_the_first_layer_knowing_the_key_and_tells_control_apart() {
        let screen = [Binding::new(Command::NextFile, &[char('n')])];
        let fallback = [Binding::new(Command::Quit, &[char('n'), ctrl('d')])];

        assert_eq!(resolve([&screen[..], &fallback[..]], &KeyEvent::from(KeyCode::Char('n'))), Some(Command::NextFile));
        assert_eq!(resolve([&screen[..], &fallback[..]], &KeyEvent::from(KeyCode::Char('d'))), None);
        assert_eq!(resolve([&screen[..], &fallback[..]], &KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)), Some(Command::Quit));
    }

    #[test]
    fn an_uppercase_letter_matches_with_shift_held() {
        assert!(char('G').matches(&KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)));
    }
}
