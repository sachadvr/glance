use std::rc::Rc;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, Paragraph};
use ratatui::Frame;

use super::{back_hit, DiffView, ListCursor, BACK};
use crate::app::command::Command;
use crate::app::keymap::{self, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Paint, Transition, View};
use crate::domain::{Commit, FileDiff};
use crate::i18n::Strings;
use crate::ui::{draw_list, grey, stat_line, tests_separator, Hit, StatColumns};

/// The files of one commit, as `git show --stat` lists them.
pub struct CommitFilesView {
    /// A copy taken when the commit opened: a reload of the branch does not pull it from under the screen.
    commit: Commit,
    cursor: ListCursor,
}

impl CommitFilesView {
    /// `commit` comes with its files loaded.
    pub fn new(commit: Commit) -> CommitFilesView {
        CommitFilesView { commit, cursor: ListCursor::default() }
    }

    fn files(&self) -> &[FileDiff] {
        self.commit.files.as_deref().unwrap_or_default()
    }
}

impl View for CommitFilesView {
    fn keymap(&self, strings: &'static Strings) -> Vec<Binding> {
        let mut bindings = keymap::list_motions();
        bindings.push(keymap::open(strings, strings.open_diff));
        bindings.push(keymap::back(strings));
        bindings
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        let length = self.files().len();
        match command {
            Command::Move(motion) => self.cursor.apply(motion, length),
            Command::Open if length > 0 => {
                let files = self.commit.files.clone().unwrap_or_else(|| Rc::from([]));
                return Transition::Push(Box::new(DiffView::new(files, self.cursor.within(length), Some(self.commit.revision.clone()), context.theme)));
            }
            Command::Back => return Transition::Pop,
            _ => {}
        }
        Transition::Stay
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        let theme = paint.theme;
        let [title, list] = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        self.cursor.drawn(list);
        let hash = self.commit.revision.as_str();
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![grey(BACK, theme), Span::styled(format!("{hash} "), Style::new().fg(theme.accent)), Span::raw(self.commit.subject.clone()).bold()]),
                Line::from(grey(format!("   {}", self.commit.byline().join(" · ")), theme)),
            ]),
            title,
        );
        let files = self.files();
        let width = usize::from(area.width).saturating_sub(1);
        let columns = StatColumns::fit(files, width, 0);
        let test_count = files.iter().filter(|file| file.is_test()).count();
        let mut items = Vec::new();
        let mut targets = Vec::new();
        let mut selected_row = 0;
        for (index, file) in files.iter().enumerate() {
            if file.is_test() && (index == 0 || !files[index - 1].is_test()) {
                items.push(tests_separator(test_count, width, theme, paint.strings));
                targets.push(None);
            }
            if index == self.cursor.index {
                selected_row = items.len();
            }
            items.push(ListItem::new(stat_line(file, columns, theme)));
            targets.push(Some(index));
        }
        let mut hits = draw_list(frame, list, items, selected_row, &targets, theme);
        hits.push(back_hit(title));
        hits
    }

    fn search_texts(&self, _session: &Session) -> Vec<String> {
        self.files().iter().map(|file| file.path.clone()).collect()
    }

    fn position(&self) -> usize {
        self.cursor.index
    }

    fn select(&mut self, index: usize) {
        self.cursor.index = index;
    }

    fn yank(&self, _session: &Session, _whole: bool) -> Option<String> {
        Some(self.commit.revision.as_str().to_string())
    }
}
