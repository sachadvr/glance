//! The app: tabs of screens, the keys and clicks they get, and what they leave behind.

mod chrome;
pub mod command;
pub mod keymap;
pub mod session;
pub mod view;
pub mod views;

use std::sync::mpsc::{self, Receiver};
use std::thread;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use ratatui::Frame;

use crate::domain::Blame;
use crate::git::{Base, Repo};
use crate::i18n::Strings;
use crate::input::Document;
use crate::search::next_match;
use crate::theme::ThemeSelection;
use crate::ui::Hit;
use command::{Command, Motion};
use keymap::{char, key, Binding};
use session::{FetchState, Session, Source};
use view::{Context, Effect, Settings, Transition, View};
use views::{BasePickerView, BlameView, CommitsView, FilesView};

const NOTICE_TEXT_CHARS: usize = 40;

/// The line under the screen: an error in red, a confirmation in green.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Error(String),
    Notice(String),
}

#[derive(Default)]
struct Search {
    /// The query being typed after `/`, while the prompt is open.
    input: Option<String>,
    /// The confirmed query: its matches are highlighted and n/N step through them.
    query: Option<String>,
}

/// A tab: its first screen, and the screens opened from it on top.
type Stack = Vec<Box<dyn View>>;

pub struct App {
    session: Session,
    tabs: Vec<Stack>,
    active: usize,
    settings: Settings,
    theme: ThemeSelection,
    strings: &'static Strings,
    search: Search,
    status: Option<Status>,
    hits: Vec<Hit>,
    hover: Option<Position>,
    fetch_done: Option<Receiver<bool>>,
    clipboard: Option<String>,
    should_quit: bool,
}

impl App {
    fn new(session: Session, tabs: Vec<Box<dyn View>>, theme: ThemeSelection, strings: &'static Strings) -> App {
        App {
            session,
            tabs: tabs.into_iter().map(|root| vec![root]).collect(),
            active: 0,
            settings: Settings::default(),
            theme,
            strings,
            search: Search::default(),
            status: None,
            hits: Vec::new(),
            hover: None,
            fetch_done: None,
            clipboard: None,
            should_quit: false,
        }
    }

    /// Browsing the repository glance was started in: its uncommitted work and its branch's commits.
    pub fn for_repo(repo: Repo, theme: ThemeSelection, strings: &'static Strings) -> App {
        let mut app = App::new(Session::new(Source::Repo(repo)), vec![Box::new(FilesView::default()), Box::new(CommitsView::default())], theme, strings);
        app.reload();
        app.execute(Command::Fetch);
        app
    }

    /// Paging what git piped. `blamed_path` names the file of a blame, which the blame itself does not.
    pub fn for_document(document: Document, blamed_path: Option<String>, theme: ThemeSelection, strings: &'static Strings) -> App {
        let (command, root): (String, Box<dyn View>) = match &document {
            Document::Diff(_) => ("git diff".to_string(), Box::new(FilesView::default())),
            Document::Log(commits) if commits.first().is_some_and(|commit| commit.is_stash()) => ("git stash".to_string(), Box::new(CommitsView::default())),
            Document::Log(_) => ("git log".to_string(), Box::new(CommitsView::default())),
            Document::Blame(_) => (format!("git blame  {}", blamed_path.as_deref().unwrap_or("")), Box::new(BlameView::default())),
        };
        let mut session = Session::new(Source::Pager { command });
        match document {
            Document::Diff(files) => session.set_files(files),
            Document::Log(commits) => session.commits = commits,
            Document::Blame(lines) => session.blame = Blame { path: blamed_path, lines },
        }
        let mut app = App::new(session, vec![root], theme, strings);
        // `git show` sends one commit: its files are what there is to look at.
        if app.session.commits.len() == 1 {
            app.execute(Command::Open);
        }
        app
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Text waiting for the runtime to hand it to the terminal's clipboard.
    pub fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    pub fn hovered(&self) -> Option<&Hit> {
        let hover = self.hover?;
        self.hits.iter().find(|hit| hit.area.contains(hover))
    }

    fn stack(&self) -> &Stack {
        &self.tabs[self.active]
    }

    fn top(&self) -> &dyn View {
        self.stack().last().expect("a tab keeps its first screen").as_ref()
    }

    fn top_mut(&mut self) -> &mut Box<dyn View> {
        self.tabs[self.active].last_mut().expect("a tab keeps its first screen")
    }

    fn is_at_root(&self) -> bool {
        self.stack().len() == 1
    }

    fn reload(&mut self) {
        if let Err(error) = self.session.reload() {
            self.status = Some(Status::Error(self.strings.git_error(&error)));
        }
    }

    /// Checks whether the background fetch finished; the main line may have moved, and the branch with it.
    pub fn poll_background(&mut self) {
        let Some(succeeded) = self.fetch_done.as_ref().and_then(|receiver| receiver.try_recv().ok()) else {
            return;
        };
        self.fetch_done = None;
        self.session.fetch = if succeeded { FetchState::Done } else { FetchState::Failed };
        if succeeded {
            self.reload();
        }
    }

    fn start_fetch(&mut self) {
        if self.session.repo().is_none() || self.session.fetch == FetchState::Running {
            return;
        }
        let git = self.session.git();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(git.fetch().is_ok());
        });
        self.session.fetch = FetchState::Running;
        self.fetch_done = Some(receiver);
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let theme = self.theme.current();
        let top = self.tabs[self.active].last_mut().expect("a tab keeps its first screen");
        let mut effects = Vec::new();
        top.prepare(&mut Context { session: &mut self.session, settings: &mut self.settings, theme, strings: self.strings, effects: &mut effects });
        self.apply(effects);
        self.hits = chrome::draw(frame, self);
    }

    /// The keys the root of a tab adds: switching tabs, and what the repository can do.
    fn root_bindings(&self) -> Vec<Binding> {
        let mut bindings = vec![Binding::new(Command::Quit, &[key(KeyCode::Esc)])];
        let titles = self.tabs.iter().enumerate().filter_map(|(index, stack)| stack[0].tab_title(&self.session, self.strings).map(|title| (index, title)));
        for (index, title) in titles {
            let binding = Binding::new(Command::SwitchTab(index), &[char(title.key)]);
            bindings.push(if self.tabs.len() > 1 { binding.hint(title.key.to_string(), self.strings.tab) } else { binding });
        }
        bindings.push(Binding::new(Command::NextTab, &[key(KeyCode::Tab)]));
        let Some(repo) = self.session.repo() else {
            return bindings;
        };
        // The base keys work from every tab, but are named only where the base row shows.
        let named = |binding: Binding, keys: &'static str| if self.top().shows_base() { binding.hint(keys, self.strings.base) } else { binding };
        bindings.push(named(Binding::new(Command::SetBase(Base::Worktree), &[char('1')]), "1"));
        if repo.mainline.is_some() {
            bindings.push(named(Binding::new(Command::SetBase(Base::Mainline), &[char('2')]), "2"));
        }
        bindings.push(named(Binding::new(Command::PickBase, &[char('3')]), "3"));
        bindings.push(Binding::new(Command::Reload, &[char('r')]).hint("r", self.strings.reload));
        bindings.push(Binding::new(Command::Fetch, &[char('F')]).hint("F", self.strings.fetch));
        bindings
    }

    /// Every layer of keys in force, the first to know a key winning.
    fn layers(&self) -> Vec<Vec<Binding>> {
        let mut layers = Vec::new();
        if self.search.query.is_some() {
            layers.push(keymap::search());
        }
        layers.push(self.top().keymap(self.strings));
        if self.is_at_root() {
            layers.push(self.root_bindings());
        }
        layers.push(keymap::global(self.strings));
        layers
    }

    pub fn on_key(&mut self, event: KeyEvent) {
        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }
        if self.search.input.is_some() {
            self.on_prompt_key(event);
            return;
        }
        self.status = None;
        let layers = self.layers();
        if let Some(command) = keymap::resolve(layers.iter().map(Vec::as_slice), &event) {
            self.execute(command);
        }
    }

    fn on_prompt_key(&mut self, event: KeyEvent) {
        let Some(mut input) = self.search.input.take() else {
            return;
        };
        match event.code {
            KeyCode::Enter => self.confirm_search(input),
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                input.pop();
                self.search.input = Some(input);
            }
            KeyCode::Char(character) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                input.push(character);
                self.search.input = Some(input);
            }
            _ => self.search.input = Some(input),
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        let position = Position { x: mouse.column, y: mouse.row };
        match mouse.kind {
            MouseEventKind::Moved => self.hover = Some(position),
            MouseEventKind::ScrollDown => self.scroll(Motion::Down),
            MouseEventKind::ScrollUp => self.scroll(Motion::Up),
            // A click while the search prompt is open would confirm it: clicks wait for the prompt to close.
            MouseEventKind::Down(MouseButton::Left) if self.search.input.is_none() => {
                if let Some(command) = self.hits.iter().find(|hit| hit.area.contains(position)).map(|hit| hit.command.clone()) {
                    self.status = None;
                    self.execute(command);
                }
            }
            _ => {}
        }
    }

    // The wheel acts as a key would: it waits for the prompt to close, and clears the last message.
    fn scroll(&mut self, motion: Motion) {
        if self.search.input.is_some() {
            return;
        }
        self.status = None;
        for _ in 0..self.top().wheel_rows() {
            self.execute(Command::Move(motion));
        }
    }

    pub fn execute(&mut self, command: Command) {
        match command {
            Command::Quit => self.should_quit = true,
            Command::CycleTheme => self.cycle_theme(),
            Command::StartSearch => self.search.input = Some(String::new()),
            Command::SearchNext => self.jump(true, false),
            Command::SearchPrevious => self.jump(false, false),
            Command::ClearSearch => self.search.query = None,
            Command::Yank => self.yank(false),
            Command::YankAll => self.yank(true),
            Command::SwitchTab(index) if index < self.tabs.len() => self.switch_tab(index),
            Command::SwitchTab(_) => {}
            Command::NextTab => self.switch_tab((self.active + 1) % self.tabs.len()),
            Command::Reload => self.reload(),
            Command::Fetch => self.start_fetch(),
            Command::SetBase(base) => self.set_base(base),
            Command::PickBase => self.pick_base(),
            Command::OpenAt(index) => {
                self.top_mut().select(index);
                self.handle(Command::Open);
            }
            command => self.handle(command),
        }
    }

    // Hands the command to the screen on top, then follows where it leads.
    fn handle(&mut self, command: Command) {
        let theme = self.theme.current();
        let mut effects = Vec::new();
        let top = self.tabs[self.active].last_mut().expect("a tab keeps its first screen");
        let transition = top.handle(command, &mut Context { session: &mut self.session, settings: &mut self.settings, theme, strings: self.strings, effects: &mut effects });
        match transition {
            Transition::Stay => {}
            Transition::Push(view) => self.push_screen(view),
            // A tab's first screen has nowhere to go back to: it quits through the root layer's Esc instead.
            Transition::Pop | Transition::Return(_) if self.is_at_root() => {}
            Transition::Pop => self.pop_screen(),
            Transition::Return(index) => {
                self.pop_screen();
                self.top_mut().select(index);
            }
        }
        self.apply(effects);
    }

    // A search belongs to the screen it was typed on: opening or leaving one ends it.
    fn push_screen(&mut self, view: Box<dyn View>) {
        self.search.query = None;
        self.tabs[self.active].push(view);
    }

    fn pop_screen(&mut self) {
        self.search.query = None;
        self.tabs[self.active].pop();
    }

    fn apply(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Error(message) => self.status = Some(Status::Error(message)),
                Effect::SetBase(base) => self.set_base(base),
                Effect::ContentChanged => self.search.query = None,
            }
        }
    }

    fn switch_tab(&mut self, index: usize) {
        if index != self.active {
            self.search.query = None;
        }
        self.active = index;
    }

    fn cycle_theme(&mut self) {
        self.theme.cycle();
        let theme = self.theme.current();
        // A screen keeps the colours it computed when it opened: they follow the new theme's.
        self.tabs.iter_mut().flatten().for_each(|view| view.restyle(theme));
    }

    fn base_tab(&self) -> Option<usize> {
        self.tabs.iter().position(|stack| stack[0].shows_base())
    }

    fn set_base(&mut self, base: Base) {
        if let Err(error) = self.session.set_base(base) {
            self.status = Some(Status::Error(self.strings.git_error(&error)));
        }
        if let Some(tab) = self.base_tab() {
            self.tabs[tab][0].select(0);
        }
    }

    fn pick_base(&mut self) {
        if self.session.commits.is_empty() {
            self.status = Some(Status::Error(self.strings.no_commit_on_branch.to_string()));
            return;
        }
        let Some(tab) = self.base_tab() else {
            return;
        };
        self.switch_tab(tab);
        if self.is_at_root() {
            self.push_screen(Box::new(BasePickerView::default()));
        }
    }

    fn confirm_search(&mut self, query: String) {
        if query.is_empty() {
            self.search.query = None;
            return;
        }
        self.search.query = Some(query);
        // Confirming looks at the cursor's own item too: a diff opens on the row it is already showing.
        self.jump(true, true);
    }

    // Moves to the match after the cursor (or from it, when `includes_current`), wrapping around the list.
    fn jump(&mut self, forward: bool, includes_current: bool) {
        let Some(query) = self.search.query.clone() else {
            return;
        };
        let texts = self.top().search_texts(&self.session);
        let position = self.top().position();
        let from = if includes_current { position.checked_sub(1).unwrap_or(texts.len().saturating_sub(1)) } else { position };
        match next_match(&texts, &query, from, forward) {
            Some(index) => self.top_mut().select(index),
            None => self.status = Some(Status::Error(self.strings.no_result.to_string())),
        }
    }

    fn yank(&mut self, whole: bool) {
        if let Some(text) = self.top().yank(&self.session, whole) {
            self.copy(text);
        }
    }

    fn copy(&mut self, text: String) {
        self.status = Some(Status::Notice((self.strings.copied)(&shown_start(&text))));
        self.clipboard = Some(text);
    }
}

// The notice shows the start of what was copied, on one line.
fn shown_start(text: &str) -> String {
    let flat = text.replace('\n', " ");
    let mut shown: String = flat.chars().take(NOTICE_TEXT_CHARS).collect();
    if flat.chars().count() > NOTICE_TEXT_CHARS {
        shown.push('…');
    }
    shown
}
