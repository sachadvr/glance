use std::ops::Range;

use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use super::grey;
use super::text::spans_width;
use crate::domain::{DiffRow, RowKind};
use crate::highlight::Segment;
use crate::search::match_ranges;
use crate::theme::Theme;

/// "  123   124 ": the old and new line numbers in front of each diff row.
pub const DIFF_GUTTER_WIDTH: usize = 12;

fn segment_style(segment: &Segment) -> Style {
    let (red, green, blue) = segment.color;
    Style::new().fg(Color::Rgb(red, green, blue))
}

/// Highlighted code as spans, each in its grammar colour.
pub fn colored_spans(segments: &[Segment]) -> Vec<Span<'static>> {
    segments.iter().map(|segment| Span::styled(segment.text.clone(), segment_style(segment))).collect()
}

fn text_spans(row: &DiffRow, colors: Option<&Vec<Segment>>, theme: &Theme) -> Vec<Span<'static>> {
    // Only a changed row carries emphasis, and it is tinted like the side it sits on.
    let emphasis_bg = match row.kind {
        RowKind::Removed => theme.removed_emphasis_bg,
        _ => theme.added_emphasis_bg,
    };
    match (row.kind, colors) {
        (RowKind::Hunk, _) => vec![Span::styled(row.text.clone(), Style::new().fg(theme.cyan))],
        (RowKind::Note, _) => vec![Span::styled(row.text.clone(), Style::new().fg(theme.grey).italic())],
        (_, Some(segments)) => emphasized_spans(segments.iter().map(|segment| (segment.text.as_str(), segment_style(segment))), &row.emphasis, emphasis_bg),
        (_, None) => emphasized_spans(std::iter::once((row.text.as_str(), Style::new())), &row.emphasis, emphasis_bg),
    }
}

// Splits styled pieces of one text where the emphasised ranges start and end, which keeps each piece's fg.
// The ranges are sorted and never overlap, as `diff_rows` produces them.
fn emphasized_spans<'a>(pieces: impl Iterator<Item = (&'a str, Style)>, emphasis: &[Range<usize>], emphasis_bg: Color) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut piece_start = 0;
    for (piece, style) in pieces {
        let piece_end = piece_start + piece.len();
        let mut cursor = piece_start;
        for range in emphasis {
            let start = range.start.max(cursor);
            let end = range.end.min(piece_end);
            if start >= end {
                continue;
            }
            if cursor < start {
                spans.push(Span::styled(piece[cursor - piece_start..start - piece_start].to_string(), style));
            }
            spans.push(Span::styled(piece[start - piece_start..end - piece_start].to_string(), style.bg(emphasis_bg)));
            cursor = end;
        }
        if cursor < piece_end {
            spans.push(Span::styled(piece[cursor - piece_start..].to_string(), style));
        }
        piece_start = piece_end;
    }
    spans
}

// Paints the matched bytes of a row's spans in the accent colour. The spans' texts, end to end, make up the row.
fn highlight_matches(spans: Vec<Span<'static>>, ranges: &[Range<usize>], theme: &Theme) -> Vec<Span<'static>> {
    if ranges.is_empty() {
        return spans;
    }
    let mut highlighted = Vec::new();
    let mut offset = 0;
    for span in spans {
        let text = span.content.to_string();
        let mut piece_start = 0;
        let mut piece_matched = false;
        for (index, _) in text.char_indices() {
            let matched = ranges.iter().any(|range| range.contains(&(offset + index)));
            if matched != piece_matched {
                push_piece(&mut highlighted, span.style, &text[piece_start..index], piece_matched, theme);
                piece_start = index;
                piece_matched = matched;
            }
        }
        push_piece(&mut highlighted, span.style, &text[piece_start..], piece_matched, theme);
        offset += text.len();
    }
    highlighted
}

fn push_piece(spans: &mut Vec<Span<'static>>, style: Style, text: &str, matched: bool, theme: &Theme) {
    if text.is_empty() {
        return;
    }
    let style = if matched { style.bg(theme.accent).fg(theme.on_accent) } else { style };
    spans.push(Span::styled(text.to_string(), style));
}

/// The rows a diff line takes: one, or as many as its text needs when long lines wrap. Continuations keep the
/// gutter's width blank so the code stays in one column.
pub fn diff_lines(row: &DiffRow, colors: Option<&Vec<Segment>>, width: usize, search: Option<&str>, wraps: bool, theme: &Theme) -> Vec<Line<'static>> {
    let number = |side: Option<usize>| side.map_or_else(|| "     ".to_string(), |line| format!("{line:>5}"));
    let (marker, background) = match row.kind {
        RowKind::Added => (Span::styled("+", Style::new().fg(theme.green)), Some(theme.added_bg)),
        RowKind::Removed => (Span::styled("-", Style::new().fg(theme.red)), Some(theme.removed_bg)),
        RowKind::Context => (Span::raw(" "), None),
        RowKind::Hunk => (Span::raw(""), Some(theme.hunk_bg)),
        RowKind::Note => (Span::raw(""), None),
    };
    let gutter = grey(format!("{} {} ", number(row.old), number(row.new)), theme);
    let matches = search.map(|query| match_ranges(&row.text, query)).unwrap_or_default();
    let mut content = vec![marker];
    content.extend(highlight_matches(text_spans(row, colors, theme), &matches, theme));
    let pieces = if wraps { wrap_spans(content, width.saturating_sub(DIFF_GUTTER_WIDTH).max(1)) } else { vec![content] };

    pieces
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            let mut spans = vec![if index == 0 { gutter.clone() } else { Span::raw(" ".repeat(DIFF_GUTTER_WIDTH)) }];
            spans.extend(piece);
            // A background only paints the cells text covers: the row is padded so the band runs to the edge.
            let gap = width.saturating_sub(spans_width(&spans));
            spans.push(Span::raw(" ".repeat(gap)));
            let line = Line::from(spans);
            match background {
                Some(color) => line.bg(color),
                None => line,
            }
        })
        .collect()
}

// Cuts styled pieces into rows of at most `width` cells, each piece keeping its style across the cut.
fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Vec<Span<'static>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut used = 0;
    for span in spans {
        let mut piece = String::new();
        for c in span.content.chars() {
            let cells = c.width().unwrap_or(0);
            if used + cells > width && used > 0 {
                if !piece.is_empty() {
                    row.push(Span::styled(std::mem::take(&mut piece), span.style));
                }
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
            piece.push(c);
            used += cells;
        }
        if !piece.is_empty() {
            row.push(Span::styled(piece, span.style));
        }
    }
    rows.push(row);
    rows
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::{Paragraph, Widget};

    use super::*;
    use crate::theme::ThemeSelection;

    fn theme() -> &'static Theme {
        ThemeSelection::default().current()
    }

    #[test]
    fn wrap_spans_cuts_at_the_width_and_keeps_each_style() {
        let bold = Style::new().bold();
        let rows = wrap_spans(vec![Span::raw("abcd"), Span::styled("efgh", bold)], 3);
        let read: Vec<Vec<(&str, bool)>> = rows.iter().map(|row| row.iter().map(|span| (span.content.as_ref(), span.style == bold)).collect()).collect();
        assert_eq!(read, [vec![("abc", false)], vec![("d", false), ("ef", true)], vec![("gh", true)]]);
    }

    #[test]
    fn highlight_matches_splits_spans_where_a_match_starts_and_ends() {
        let spans = vec![Span::raw("ab"), Span::raw("Cd")];
        let pieces = highlight_matches(spans, &[1..2, 3..4], theme());
        let painted: Vec<(&str, bool)> = pieces.iter().map(|piece| (piece.content.as_ref(), piece.style.bg == Some(theme().accent))).collect();
        assert_eq!(painted, [("a", false), ("b", true), ("C", false), ("d", true)]);
    }

    #[test]
    fn emphasised_words_take_the_stronger_background_over_the_row_band() {
        // The gutter is 12 cells and the marker one more: the text starts at column 13.
        let mut row = DiffRow::new(RowKind::Removed, Some(1), None, "let x = foo;");
        row.emphasis.push(8..11);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));

        Paragraph::new(diff_lines(&row, None, 30, None, false, theme()).remove(0)).render(buffer.area, &mut buffer);

        assert_eq!(buffer[(20, 0)].bg, theme().removed_bg);
        assert_eq!(buffer[(21, 0)].bg, theme().removed_emphasis_bg);
        assert_eq!(buffer[(23, 0)].bg, theme().removed_emphasis_bg);
        assert_eq!(buffer[(24, 0)].bg, theme().removed_bg);
    }

    #[test]
    fn diff_lines_wraps_with_a_blank_gutter_and_pads_each_row_to_the_width() {
        let row = DiffRow::new(RowKind::Added, None, Some(7), "abcdefgh");

        let lines = diff_lines(&row, None, DIFF_GUTTER_WIDTH + 5, None, true, theme());

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].spans[0].content, " ".repeat(DIFF_GUTTER_WIDTH));
        assert!(lines.iter().all(|line| line.width() == DIFF_GUTTER_WIDTH + 5));
    }

    #[test]
    fn colored_spans_carry_each_segment_colour() {
        let spans = colored_spans(&[Segment { color: (1, 2, 3), text: "fn".to_string() }]);

        assert_eq!(spans, [Span::styled("fn", Style::new().fg(Color::Rgb(1, 2, 3)))]);
    }
}
