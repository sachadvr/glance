use std::rc::Rc;

use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::BACK;
use crate::app::command::{Command, Motion};
use crate::app::keymap::{self, char, ctrl, key, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Effect, Paint, Transition, View};
use crate::domain::{diff_rows, DiffRow, FileDiff};
use crate::highlight::{highlight, Segment};
use crate::theme::Theme;
use crate::ui::{diff_lines, grey, spans_width, stat_spans, status_color, Hit, Row};

const WHEEL_ROWS: usize = 3;

/// One file's diff, among the files it was opened from: n/p step to its neighbours.
pub struct DiffView {
    files: Rc<[FileDiff]>,
    index: usize,
    scroll: usize,
    /// The row the last search landed on: near the end it sits below the top row, which cannot scroll further.
    found: Option<usize>,
    /// Rows the body showed at the last draw: what a page key moves by.
    page: usize,
    rows: Vec<DiffRow>,
    colors: Vec<Option<Vec<Segment>>>,
}

impl DiffView {
    pub fn new(files: Rc<[FileDiff]>, index: usize, theme: &Theme) -> DiffView {
        let mut view = DiffView { files, index, scroll: 0, found: None, page: 20, rows: Vec::new(), colors: Vec::new() };
        view.load(index, theme);
        view
    }

    fn file(&self) -> &FileDiff {
        &self.files[self.index]
    }

    fn show(&mut self, index: usize, context: &mut Context) {
        self.load(index, context.theme);
        context.effects.push(Effect::ContentChanged);
    }

    fn load(&mut self, index: usize, theme: &Theme) {
        self.index = index;
        self.scroll = 0;
        self.found = None;
        self.rows = diff_rows(self.file());
        self.colors = highlight(&self.file().path, &self.rows, theme.syntax);
    }

    // The last page stays full: scrolling stops when the last row reaches the bottom.
    fn last_scroll(&self) -> usize {
        self.rows.len().saturating_sub(self.page)
    }
}

impl View for DiffView {
    fn keymap(&self, _session: &Session) -> Vec<Binding> {
        vec![
            Binding::new(Command::Move(Motion::Down), &[key(KeyCode::Down), char('j'), key(KeyCode::Enter)]).hint("j", "défiler"),
            Binding::new(Command::Move(Motion::Up), &[key(KeyCode::Up), char('k')]).hint("k", "défiler"),
            Binding::new(Command::Move(Motion::PageDown), &[key(KeyCode::PageDown), char(' ')]).hint("espace", "page"),
            Binding::new(Command::Move(Motion::PageUp), &[key(KeyCode::PageUp), char('b')]).hint("b", "page"),
            Binding::new(Command::Move(Motion::HalfPageDown), &[ctrl('d')]),
            Binding::new(Command::Move(Motion::HalfPageUp), &[ctrl('u')]),
            Binding::new(Command::Move(Motion::Top), &[char('g'), key(KeyCode::Home)]),
            Binding::new(Command::Move(Motion::Bottom), &[char('G'), key(KeyCode::End)]),
            Binding::new(Command::NextFile, &[char('n'), char(']')]).hint("n", "fichier"),
            Binding::new(Command::PreviousFile, &[char('p'), char('[')]).hint("p", "fichier"),
            Binding::new(Command::ToggleWrap, &[char('w')]).hint("w", "retour ligne"),
            Binding::new(Command::YankAll, &[char('Y')]).hint("Y", "copier"),
            keymap::back(),
        ]
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        match command {
            Command::Move(motion) => {
                self.scroll = motion.apply(self.scroll, self.last_scroll() + 1, self.page);
                self.found = None;
            }
            Command::NextFile if self.index + 1 < self.files.len() => self.show(self.index + 1, context),
            Command::PreviousFile if self.index > 0 => self.show(self.index - 1, context),
            Command::ToggleWrap => context.settings.wraps_lines = !context.settings.wraps_lines,
            Command::Back => return Transition::Return(self.index),
            _ => {}
        }
        Transition::Stay
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        let [title, body] = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        self.page = usize::from(body.height).max(1);
        let width = usize::from(area.width);
        let heading = self.heading(width, paint.theme);
        let hits = heading.hits(title, 0).collect();
        frame.render_widget(Paragraph::new(vec![Line::from(heading.spans), Line::from(grey("─".repeat(width), paint.theme))]), title);
        let lines: Vec<Line> = self
            .rows
            .iter()
            .zip(&self.colors)
            .skip(self.scroll)
            .flat_map(|(row, colors)| diff_lines(row, colors.as_ref(), width, paint.search, paint.settings.wraps_lines, paint.theme))
            .take(usize::from(body.height))
            .collect();
        frame.render_widget(Paragraph::new(lines), body);
        hits
    }

    fn restyle(&mut self, theme: &Theme) {
        self.colors = highlight(&self.file().path, &self.rows, theme.syntax);
    }

    fn wheel_rows(&self) -> usize {
        WHEEL_ROWS
    }

    // A diff's position is its top row: a search starts from there, as `less` does.
    fn search_texts(&self, _session: &Session) -> Vec<String> {
        self.rows.iter().map(|row| row.text.clone()).collect()
    }

    fn position(&self) -> usize {
        self.found.unwrap_or(self.scroll)
    }

    // A match near the end stays on screen with the last page, rather than above blank rows.
    fn select(&mut self, index: usize) {
        self.scroll = index.min(self.last_scroll());
        self.found = Some(index);
    }

    fn yank(&self, _session: &Session, whole: bool) -> Option<String> {
        let file = self.file();
        Some(if whole { file.lines.join("\n") } else { file.path.clone() })
    }
}

impl DiffView {
    // "← M src/a.php +3 −1        ◀ préc  2/5  suiv ▶", the navigation against the right edge.
    fn heading(&self, width: usize, theme: &Theme) -> Row {
        let file = self.file();
        let mut heading = Row::default();
        heading.push_target(grey(BACK, theme), Command::Back);
        heading.push(Span::styled(file.status.letter().to_string(), Style::new().fg(status_color(file.status, theme)).bold()));
        heading.push(Span::raw(format!(" {}", file.path)).bold());
        stat_spans(file.stats, theme).into_iter().for_each(|span| heading.push(span));
        if let Some(old_path) = &file.old_path {
            heading.push(grey(format!("   {} depuis {old_path}", file.status.label()), theme));
        }
        let enabled = |is_enabled: bool| Style::new().fg(if is_enabled { theme.accent } else { theme.grey });
        let previous = Span::styled("◀ préc", enabled(self.index > 0));
        let position = grey(format!("  {}/{}  ", self.index + 1, self.files.len()), theme);
        let next = Span::styled("suiv ▶ ", enabled(self.index + 1 < self.files.len()));
        let navigation_width = spans_width(&[previous.clone(), position.clone(), next.clone()]);
        heading.push(Span::raw(" ".repeat(width.saturating_sub(heading.width() + navigation_width).max(3))));
        heading.push_target(previous, Command::PreviousFile);
        heading.push(position);
        heading.push_target(next, Command::NextFile);
        heading
    }
}
