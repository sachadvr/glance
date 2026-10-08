use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, Paragraph};
use ratatui::Frame;

use super::commits::commit_search_text;
use super::{back_hit, ListCursor, BACK};
use crate::app::command::Command;
use crate::app::keymap::{self, key, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Effect, Paint, Transition, View};
use crate::domain::Revision;
use crate::git::Base;
use crate::ui::{draw_list, every_row, fit_end, grey, Hit};

/// Choosing the commit the files tab counts from.
#[derive(Default)]
pub struct BasePickerView {
    cursor: ListCursor,
}

impl View for BasePickerView {
    fn keymap(&self, _session: &Session) -> Vec<Binding> {
        let mut bindings = keymap::list_motions();
        bindings.push(Binding::new(Command::Open, &[key(KeyCode::Enter)]).hint("clic/⏎", "choisir"));
        bindings.push(Binding::new(Command::Back, &[key(KeyCode::Esc), key(KeyCode::Left), key(KeyCode::Backspace)]).hint("esc", "annuler"));
        bindings
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        let commits = &context.session.commits;
        match command {
            Command::Move(motion) => self.cursor.apply(motion, commits.len()),
            Command::Open => {
                if let Some(Revision::Hash(hash)) = commits.get(self.cursor.index).map(|commit| &commit.revision) {
                    context.effects.push(Effect::SetBase(Base::Commit(hash.clone())));
                }
                return Transition::Pop;
            }
            Command::Back => return Transition::Pop,
            _ => {}
        }
        Transition::Stay
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        let theme = paint.theme;
        let [title, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
        self.cursor.drawn(list);
        frame.render_widget(Paragraph::new(Line::from(vec![grey(BACK, theme), grey("Compter les fichiers à partir du commit :", theme)])), title);
        let width = usize::from(area.width).saturating_sub(1);
        let commits = &paint.session.commits;
        let items = commits
            .iter()
            .map(|commit| {
                let hash = commit.revision.as_str();
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {hash} "), Style::new().fg(theme.accent)),
                    Span::raw(fit_end(&commit.subject, width.saturating_sub(hash.len() + 2))),
                ]))
            })
            .collect();
        let mut hits = draw_list(frame, list, items, self.cursor.within(commits.len()), &every_row(commits.len()), theme);
        hits.push(back_hit(title));
        hits
    }

    fn shows_base(&self) -> bool {
        true
    }

    fn search_texts(&self, session: &Session) -> Vec<String> {
        session.commits.iter().map(commit_search_text).collect()
    }

    fn position(&self) -> usize {
        self.cursor.index
    }

    fn select(&mut self, index: usize) {
        self.cursor.index = index;
    }

    fn yank(&self, session: &Session, _whole: bool) -> Option<String> {
        session.commits.get(self.cursor.within(session.commits.len())).map(|commit| commit.revision.as_str().to_string())
    }
}
