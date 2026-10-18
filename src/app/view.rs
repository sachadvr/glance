use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::Frame;

use super::command::Command;
use super::keymap::Binding;
use super::session::Session;
use crate::domain::ChangeStats;
use crate::git::{Base, GitError};
use crate::i18n::Strings;
use crate::theme::Theme;
use crate::ui::Hit;

/// What a screen leaves behind for the app: a message, a new base to count from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Error(String),
    SetBase(Base),
    /// The screen shows something else in place (a diff's next file): a search typed on the old content ends.
    ContentChanged,
}

/// Settings that outlive the screen that changed them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// Whether a diff wraps its long lines rather than cutting them at the edge.
    pub wraps_lines: bool,
}

/// What a screen handles a command with.
pub struct Context<'a> {
    pub session: &'a mut Session,
    pub settings: &'a mut Settings,
    pub theme: &'static Theme,
    pub strings: &'static Strings,
    pub effects: &'a mut Vec<Effect>,
}

impl Context<'_> {
    pub fn fail(&mut self, message: impl Into<String>) {
        self.effects.push(Effect::Error(message.into()));
    }

    pub fn fail_git(&mut self, error: &GitError) {
        let message = self.strings.git_error(error);
        self.fail(message);
    }
}

/// What a screen is drawn with.
pub struct Paint<'a> {
    pub session: &'a Session,
    pub settings: Settings,
    pub theme: &'static Theme,
    pub strings: &'static Strings,
    /// The confirmed search, which a screen showing text highlights.
    pub search: Option<&'a str>,
}

/// Where the app goes after a command.
pub enum Transition {
    Stay,
    Push(Box<dyn View>),
    Pop,
    /// Back to the screen beneath, its cursor on this item: a diff browsed with n/p returns to the file it ended on.
    Return(usize),
}

/// How a tab presents itself: "[c] Commits 12 +40 −3".
pub struct TabTitle {
    pub key: char,
    pub label: &'static str,
    pub count: usize,
    pub stats: ChangeStats,
}

/// A piece of the tab row a screen adds for itself, clickable when it carries a command.
pub struct Tool {
    pub span: Span<'static>,
    pub command: Command,
}

/// A screen: a list or a file, in a tab's stack. A new screen is a new implementation: the app only knows this.
pub trait View {
    fn keymap(&self, strings: &'static Strings) -> Vec<Binding>;

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition;

    /// Draws the screen and returns where its clickable areas fell.
    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit>;

    /// A tab's first screen names the tab.
    fn tab_title(&self, _session: &Session, _strings: &'static Strings) -> Option<TabTitle> {
        None
    }

    fn tools(&self, _theme: &Theme, _strings: &Strings) -> Vec<Tool> {
        Vec::new()
    }

    /// Whether the row choosing what the files are counted from shows above this screen.
    fn shows_base(&self) -> bool {
        false
    }

    /// The texts a search runs over, one per item the cursor can land on.
    fn search_texts(&self, _session: &Session) -> Vec<String> {
        Vec::new()
    }

    fn position(&self) -> usize {
        0
    }

    fn select(&mut self, _index: usize) {}

    /// Rows a turn of the mouse wheel moves by.
    fn wheel_rows(&self) -> usize {
        1
    }

    /// What `y` copies (the identifier the screen is about), or `Y` with `whole` (the whole of it).
    fn yank(&self, _session: &Session, _whole: bool) -> Option<String> {
        None
    }

    /// Called before drawing, for what must be loaded first.
    fn prepare(&mut self, _context: &mut Context) {}

    /// The theme changed: colours computed from the old one must be computed again.
    fn restyle(&mut self, _theme: &Theme) {}
}
