use std::rc::Rc;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, Paragraph};
use ratatui::Frame;

use super::{DiffView, ListCursor};
use crate::app::command::Command;
use crate::app::keymap::{self, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Paint, TabTitle, Transition, View};
use crate::domain::{total_stats, FileDiff};
use crate::i18n::Strings;
use crate::theme::Theme;
use crate::ui::{draw_list, fit_end, grey, padded, spans_width, stat_spans, status_color, tests_separator, Hit};

/// The changed files, grouped under their directories, the tests last.
#[derive(Default)]
pub struct FilesView {
    cursor: ListCursor,
}

impl View for FilesView {
    fn keymap(&self, strings: &'static Strings) -> Vec<Binding> {
        let mut bindings = keymap::list_motions();
        bindings.push(keymap::open(strings, strings.open_diff));
        bindings
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        let files = &context.session.files;
        match command {
            Command::Move(motion) => self.cursor.apply(motion, files.len()),
            Command::Open if !files.is_empty() => {
                return Transition::Push(Box::new(DiffView::new(Rc::clone(files), self.cursor.within(files.len()), context.theme)));
            }
            _ => {}
        }
        Transition::Stay
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        self.cursor.drawn(area);
        let files = &paint.session.files;
        if files.is_empty() {
            frame.render_widget(Paragraph::new(grey(format!(" {}", paint.strings.no_changes), paint.theme)), area);
            return Vec::new();
        }
        draw_files(frame, area, files, self.cursor.within(files.len()), paint)
    }

    fn tab_title(&self, session: &Session, strings: &'static Strings) -> Option<TabTitle> {
        Some(TabTitle { key: 'f', label: strings.files_tab, count: session.files.len(), stats: total_stats(&session.files) })
    }

    fn shows_base(&self) -> bool {
        true
    }

    fn search_texts(&self, session: &Session) -> Vec<String> {
        session.files.iter().map(|file| file.path.clone()).collect()
    }

    fn position(&self) -> usize {
        self.cursor.index
    }

    fn select(&mut self, index: usize) {
        self.cursor.index = index;
    }

    fn yank(&self, session: &Session, _whole: bool) -> Option<String> {
        session.files.get(self.cursor.within(session.files.len())).map(|file| file.path.clone())
    }
}

fn draw_files(frame: &mut Frame, area: Rect, files: &[FileDiff], cursor: usize, paint: &Paint) -> Vec<Hit> {
    let theme = paint.theme;
    let width = usize::from(area.width).saturating_sub(1);
    let test_count = files.iter().filter(|file| file.is_test()).count();
    let mut items = Vec::new();
    let mut targets = Vec::new();
    let mut selected_row = 0;
    let mut directory = None;
    for (index, file) in files.iter().enumerate() {
        // Files come sorted with the tests last: the first test opens their section, under its own directories.
        if file.is_test() && (index == 0 || !files[index - 1].is_test()) {
            items.push(tests_separator(test_count, width, theme, paint.strings));
            targets.push(None);
            directory = None;
        }
        if directory != Some(file.directory()) {
            directory = Some(file.directory());
            let shown = if file.directory().is_empty() { "./" } else { file.directory() };
            items.push(ListItem::new(Line::from(grey(format!(" {shown}"), theme))));
            targets.push(None);
        }
        if index == cursor {
            selected_row = items.len();
        }
        items.push(ListItem::new(file_line(file, width, theme)));
        targets.push(Some(index));
    }
    draw_list(frame, area, items, selected_row, &targets, theme)
}

fn file_line(file: &FileDiff, width: usize, theme: &Theme) -> Line<'static> {
    let stats = stat_spans(file.stats, theme);
    let name_width = width.saturating_sub(4 + spans_width(&stats) + 1);
    let mut spans = vec![
        Span::raw("  "),
        Span::styled(file.status.letter().to_string(), Style::new().fg(status_color(file.status, theme)).bold()),
        Span::raw(" "),
        Span::raw(padded(fit_end(file.name(), name_width), name_width)),
    ];
    spans.extend(stats);
    Line::from(spans)
}
