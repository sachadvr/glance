use ratatui::layout::Rect;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::text::spans_width;
use super::Hit;
use crate::app::command::Command;

/// A line being built, which remembers where its clickable spans fall.
#[derive(Default)]
pub struct Row {
    pub spans: Vec<Span<'static>>,
    // Start column, width, and what a click there runs.
    targets: Vec<(u16, u16, Command)>,
}

impl Row {
    pub fn push(&mut self, span: Span<'static>) {
        self.spans.push(span);
    }

    pub fn push_target(&mut self, span: Span<'static>, command: Command) {
        let start = self.width() as u16;
        self.targets.push((start, span.content.width() as u16, command));
        self.spans.push(span);
    }

    /// Where the clickable spans land once the row is drawn on line `line` of `area`.
    pub fn hits(&self, area: Rect, line: u16) -> impl Iterator<Item = Hit> + '_ {
        self.targets
            .iter()
            .map(move |(start, width, command)| Hit { area: Rect::new(area.x + start, area.y + line, *width, 1), command: command.clone() })
    }

    pub fn width(&self) -> usize {
        spans_width(&self.spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_place_each_target_after_the_spans_before_it() {
        let mut row = Row::default();
        row.push(Span::raw("日 "));
        row.push_target(Span::raw("open"), Command::Open);

        let hits: Vec<Hit> = row.hits(Rect::new(2, 5, 40, 3), 1).collect();

        assert_eq!(hits, [Hit { area: Rect::new(5, 6, 4, 1), command: Command::Open }]);
        assert_eq!(row.width(), 7);
    }
}
