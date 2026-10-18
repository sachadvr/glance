use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{HighlightSpacing, List, ListItem, ListState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::{grey, Hit};
use crate::app::command::Command;
use crate::i18n::Strings;
use crate::theme::Theme;

fn selection_list(items: Vec<ListItem<'static>>, theme: &Theme) -> List<'static> {
    List::new(items)
        .highlight_style(Style::new().bg(theme.selected_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("›")
        .highlight_spacing(HighlightSpacing::Always)
}

/// Renders a list and returns where each visible row that opens something sits; `targets` maps rows to the
/// index they open (`None` for a directory heading).
pub fn draw_list(frame: &mut Frame, area: Rect, items: Vec<ListItem<'static>>, selected_row: usize, targets: &[Option<usize>], theme: &Theme) -> Vec<Hit> {
    let heights: Vec<u16> = items.iter().map(|item| item.height() as u16).collect();
    let mut state = ListState::default().with_selected(Some(selected_row));
    frame.render_stateful_widget(selection_list(items, theme), area, &mut state);

    let mut hits = Vec::new();
    let mut y = area.y;
    for (target, height) in targets.iter().zip(heights).skip(state.offset()) {
        if y + height > area.bottom() {
            break;
        }
        if let Some(index) = target {
            hits.push(Hit { area: Rect::new(area.x, y, area.width, height), command: Command::OpenAt(*index) });
        }
        y += height;
    }
    hits
}

/// Targets for a list whose every row opens the item at its own index.
pub fn every_row(count: usize) -> Vec<Option<usize>> {
    (0..count).map(Some).collect()
}

/// The heading that opens the tests' section of a file list, after a blank line.
pub fn tests_separator(count: usize, width: usize, theme: &Theme, strings: &Strings) -> ListItem<'static> {
    let label = format!(" {} ", (strings.tests_section)(count));
    let rule = "─".repeat(width.saturating_sub(label.width() + 2));

    ListItem::new(vec![Line::default(), Line::from(vec![grey("──", theme), Span::styled(label, Style::new().fg(theme.accent).bold()), grey(rule, theme)])])
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::theme::ThemeSelection;

    #[test]
    fn draw_list_records_a_hit_for_each_visible_target_and_skips_headings() {
        let theme = ThemeSelection::default().current();
        let mut terminal = Terminal::new(TestBackend::new(20, 2)).expect("a test terminal");
        let items = ["dir", "a", "b"].map(ListItem::new).to_vec();
        let mut hits = Vec::new();

        terminal.draw(|frame| hits = draw_list(frame, frame.area(), items, 1, &[None, Some(0), Some(1)], theme)).expect("drawn");

        assert_eq!(hits, [Hit { area: Rect::new(0, 1, 20, 1), command: Command::OpenAt(0) }]);
    }

    #[test]
    fn tests_separator_says_file_in_the_singular_for_one_test() {
        let theme = ThemeSelection::default().current();
        let separator = tests_separator(1, 30, theme, &crate::i18n::FRENCH);

        assert_eq!(separator.height(), 2);
        assert_eq!(every_row(2), [Some(0), Some(1)]);
    }
}
