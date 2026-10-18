use std::rc::Rc;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;
use ratatui::Frame;

use super::{CommitFilesView, ListCursor};
use crate::app::command::Command;
use crate::app::keymap::{self, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Paint, TabTitle, Transition, View};
use crate::domain::{expand_tabs, Blame, BlameLine, ChangeStats, Commit, DiffRow, Revision, RowKind};
use crate::highlight::{highlight, Segment};
use crate::i18n::Strings;
use crate::theme::Theme;
use crate::ui::{colored_spans, draw_list, every_row, fit_end, grey, padded, Hit};

const AUTHOR_WIDTH: usize = 14;
// " 7bfd4f45 " + author + " 2024-08-21 ".
const GUTTER_WIDTH: usize = 10 + AUTHOR_WIDTH + 12;

/// A blamed file, line by line, each run of lines from one commit introduced by who wrote it and when.
#[derive(Default)]
pub struct BlameView {
    cursor: ListCursor,
    /// Computed before the first draw, and again after the theme changes.
    colors: Option<Vec<Option<Vec<Segment>>>>,
}

impl View for BlameView {
    fn keymap(&self, strings: &'static Strings) -> Vec<Binding> {
        let mut bindings = keymap::list_motions();
        bindings.push(keymap::open(strings, strings.open_line_commit));
        bindings
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        let length = context.session.blame.lines.len();
        match command {
            Command::Move(motion) => self.cursor.apply(motion, length),
            Command::Open if length > 0 => return open_line(context, &context.session.blame.lines[self.cursor.within(length)].clone()),
            _ => {}
        }
        Transition::Stay
    }

    fn prepare(&mut self, context: &mut Context) {
        if self.colors.is_none() {
            self.colors = Some(colors(&context.session.blame, context.theme));
        }
    }

    fn restyle(&mut self, _theme: &Theme) {
        self.colors = None;
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        self.cursor.drawn(area);
        let lines = &paint.session.blame.lines;
        let colors = self.colors.as_deref().unwrap_or_default();
        let items = lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let mut spans = gutter(line, index.checked_sub(1).map(|previous| &lines[previous]), paint);
                spans.push(grey(format!("{:>5} │ ", line.number), paint.theme));
                match colors.get(index).and_then(Option::as_ref) {
                    Some(segments) => spans.extend(colored_spans(segments)),
                    None => spans.push(Span::raw(expand_tabs(&line.code))),
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        draw_list(frame, area, items, self.cursor.within(lines.len()), &every_row(lines.len()), paint.theme)
    }

    fn tab_title(&self, session: &Session, strings: &'static Strings) -> Option<TabTitle> {
        Some(TabTitle { key: 'b', label: strings.lines_tab, count: session.blame.lines.len(), stats: ChangeStats::default() })
    }

    fn search_texts(&self, session: &Session) -> Vec<String> {
        session.blame.lines.iter().map(|line| line.code.clone()).collect()
    }

    fn position(&self) -> usize {
        self.cursor.index
    }

    fn select(&mut self, index: usize) {
        self.cursor.index = index;
    }

    fn yank(&self, session: &Session, _whole: bool) -> Option<String> {
        session.blame.lines.get(self.cursor.within(session.blame.lines.len())).map(|line| line.hash.clone())
    }
}

// The commit that last wrote the line, opened like any other commit.
fn open_line(context: &mut Context, line: &BlameLine) -> Transition {
    if !line.is_committed() {
        context.fail(context.strings.line_not_committed);
        return Transition::Stay;
    }
    let git = context.session.git();
    let revision = Revision::Hash(line.hash.clone());
    let files = match git.files_of(&revision) {
        Ok(files) => files,
        Err(error) => {
            context.fail_git(&error);
            return Transition::Stay;
        }
    };
    let mut commit = Commit::new(revision, git.subject(&line.hash).unwrap_or_default());
    commit.author = line.author.clone();
    commit.when = line.date.clone();
    commit.files = Some(Rc::from(files));
    Transition::Push(Box::new(CommitFilesView::new(commit)))
}

fn colors(blame: &Blame, theme: &Theme) -> Vec<Option<Vec<Segment>>> {
    let rows: Vec<DiffRow> = blame.lines.iter().map(|line| DiffRow::new(RowKind::Context, None, Some(line.number), expand_tabs(&line.code))).collect();
    highlight(blame.path.as_deref().unwrap_or(""), &rows, theme.syntax)
}

// Who wrote the line and when, shown once for a run of lines from one commit.
fn gutter(line: &BlameLine, previous: Option<&BlameLine>, paint: &Paint) -> Vec<Span<'static>> {
    let theme = paint.theme;
    if previous.is_some_and(|previous| previous.hash == line.hash) {
        return vec![Span::raw(" ".repeat(GUTTER_WIDTH))];
    }
    if !line.is_committed() {
        return vec![Span::styled(padded(fit_end(&format!(" {}", paint.strings.not_committed_yet), GUTTER_WIDTH), GUTTER_WIDTH), Style::new().fg(theme.green))];
    }

    vec![
        Span::styled(format!(" {} ", line.hash), Style::new().fg(theme.accent)),
        Span::raw(padded(fit_end(&line.author, AUTHOR_WIDTH), AUTHOR_WIDTH)),
        grey(format!(" {} ", line.date), theme),
    ]
}
