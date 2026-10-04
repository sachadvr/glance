use ratatui::text::Span;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Cuts the end off text wider than `width`, an ellipsis standing for what is left out.
pub fn fit_end(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut fitted = String::new();
    let mut used = 1;
    for c in text.chars() {
        used += c.width().unwrap_or(0);
        if used > width {
            break;
        }
        fitted.push(c);
    }
    fitted + "…"
}

/// Cuts the start off instead: a path reads by its end.
pub fn fit_start(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut kept: Vec<char> = Vec::new();
    let mut used = 1;
    for c in text.chars().rev() {
        used += c.width().unwrap_or(0);
        if used > width {
            break;
        }
        kept.push(c);
    }
    std::iter::once('…').chain(kept.into_iter().rev()).collect()
}

/// Fills `text` with spaces up to `width` cells, so the columns after it line up.
pub fn padded(text: String, width: usize) -> String {
    let gap = width.saturating_sub(text.width());
    text + &" ".repeat(gap)
}

/// The cells the spans take on screen, not their bytes.
pub fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|span| span.content.width()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_end_keeps_text_that_fits_and_cuts_the_end_of_text_that_does_not() {
        assert_eq!(fit_end("abc", 3), "abc");
        assert_eq!(fit_end("abcdef", 4), "abc…");
    }

    #[test]
    fn fit_end_counts_a_wide_character_as_two_cells() {
        assert_eq!(fit_end("日本語", 4), "日…");
    }

    #[test]
    fn fit_start_cuts_the_start_and_keeps_the_end() {
        assert_eq!(fit_start("src/a/b.rs", 6), "…/b.rs");
        assert_eq!(fit_start("b.rs", 6), "b.rs");
    }

    #[test]
    fn fit_start_counts_a_wide_character_as_two_cells() {
        assert_eq!(fit_start("日本語", 4), "…語");
    }

    #[test]
    fn padded_fills_up_to_the_width_in_cells() {
        assert_eq!(padded("日".to_string(), 4), "日  ");
        assert_eq!(padded("abcdef".to_string(), 4), "abcdef");
    }

    #[test]
    fn spans_width_adds_the_cells_of_every_span() {
        assert_eq!(spans_width(&[Span::raw("ab"), Span::raw("日")]), 4);
    }
}
