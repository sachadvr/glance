//! Syntax colouring of a diff's rows, with bat's grammars.

use std::path::Path;
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

use crate::domain::{DiffRow, RowKind};

// Past this, colouring a file (a lock file, a generated dump) would delay its opening for nothing worth reading.
const MAX_HIGHLIGHTED_ROWS: usize = 5000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub color: (u8, u8, u8),
    pub text: String,
}

struct Highlighter {
    syntaxes: SyntaxSet,
    // Each theme is only deserialized the first time it is picked.
    themes: EmbeddedLazyThemeSet,
}

// bat's grammars and themes: syntect's own set has no TypeScript, Vue or Dockerfile.
fn highlighter() -> &'static Highlighter {
    static HIGHLIGHTER: OnceLock<Highlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(|| Highlighter { syntaxes: two_face::syntax::extra_newlines(), themes: two_face::theme::extra() })
}

fn syntax_for<'a>(syntaxes: &'a SyntaxSet, path: &str) -> Option<&'a SyntaxReference> {
    let path = Path::new(path);
    let extension = path.extension().and_then(|extension| extension.to_str());
    let name = path.file_name().and_then(|name| name.to_str());

    // "Dockerfile" or "Makefile" have no extension: grammars list those names among their extensions.
    let syntax = extension
        .and_then(|extension| syntaxes.find_syntax_by_extension(extension))
        .or_else(|| name.and_then(|name| syntaxes.find_syntax_by_extension(name)))?;
    // The PHP grammar starts as HTML and switches to PHP at "<?php", which a hunk rarely contains: the code
    // itself is read with the grammar it embeds.
    if syntax.name == "PHP" {
        return syntaxes.find_syntax_by_name("PHP Source").or(Some(syntax));
    }

    Some(syntax)
}

fn segments(highlighter: &mut HighlightLines, text: &str, syntaxes: &SyntaxSet) -> Option<Vec<Segment>> {
    let line = format!("{text}\n");
    let ranges = highlighter.highlight_line(&line, syntaxes).ok()?;

    Some(
        ranges
            .into_iter()
            .map(|(style, piece)| Segment { color: (style.foreground.r, style.foreground.g, style.foreground.b), text: piece.trim_end_matches('\n').to_string() })
            .filter(|segment| !segment.text.is_empty())
            .collect(),
    )
}

/// The colours of each row's text, `None` for a row drawn plain (hunk headers, notes, unknown languages).
pub fn highlight(path: &str, rows: &[DiffRow], syntax: EmbeddedThemeName) -> Vec<Option<Vec<Segment>>> {
    let highlighter = highlighter();
    let grammar = syntax_for(&highlighter.syntaxes, path).filter(|_| rows.len() <= MAX_HIGHLIGHTED_ROWS);
    let Some(grammar) = grammar else {
        return vec![None; rows.len()];
    };
    // Each side of a diff is a file of its own: a removed line parses after the old side's lines, an added one
    // after the new side's, so an opened string or comment on one side does not leak into the other.
    let colours = highlighter.themes.get(syntax);
    let mut old_side = HighlightLines::new(grammar, colours);
    let mut new_side = HighlightLines::new(grammar, colours);
    let syntaxes = &highlighter.syntaxes;

    rows.iter()
        .map(|row| match row.kind {
            RowKind::Removed => segments(&mut old_side, &row.text, syntaxes),
            RowKind::Added => segments(&mut new_side, &row.text, syntaxes),
            RowKind::Context => {
                segments(&mut old_side, &row.text, syntaxes);
                segments(&mut new_side, &row.text, syntaxes)
            }
            RowKind::Hunk | RowKind::Note => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTAX: EmbeddedThemeName = EmbeddedThemeName::CatppuccinMocha;

    fn row(kind: RowKind, text: &str) -> DiffRow {
        DiffRow::new(kind, None, None, text)
    }

    fn colours_of(segments: &Option<Vec<Segment>>) -> Vec<(u8, u8, u8)> {
        segments.as_ref().expect("the row is highlighted").iter().map(|segment| segment.color).collect()
    }

    #[test]
    fn highlight_colours_a_known_language_and_keeps_the_text() {
        let rows = [row(RowKind::Hunk, "@@ -1 +1 @@"), row(RowKind::Added, "fn main() {}")];

        let colours = highlight("src/main.rs", &rows, SYNTAX);

        assert_eq!(colours[0], None);
        let added = colours[1].as_ref().expect("Rust is highlighted");
        assert_eq!(added.iter().map(|segment| segment.text.as_str()).collect::<String>(), "fn main() {}");
        assert!(added.len() > 1);
    }

    #[test]
    fn highlight_colours_php_without_its_opening_tag() {
        let colours = highlight("src/Service.php", &[row(RowKind::Added, "$promoted = $this->collect($dto);")], SYNTAX);

        let distinct: std::collections::HashSet<_> = colours_of(&colours[0]).into_iter().collect();
        assert!(distinct.len() > 2);
    }

    #[test]
    fn highlight_leaves_an_unknown_language_plain() {
        assert_eq!(highlight("notes.unknownext", &[row(RowKind::Added, "x")], SYNTAX), vec![None]);
    }

    #[test]
    fn highlight_recognises_an_extensionless_file_by_its_name() {
        let colours = highlight("docker/Dockerfile", &[row(RowKind::Added, "FROM rust:1.80")], SYNTAX);

        assert!(colours[0].is_some());
    }

    #[test]
    fn highlight_leaves_a_file_past_the_row_limit_plain() {
        let rows = vec![row(RowKind::Added, "fn main() {}"); MAX_HIGHLIGHTED_ROWS + 1];

        assert!(highlight("src/main.rs", &rows, SYNTAX).iter().all(Option::is_none));
    }

    #[test]
    fn highlight_leaves_notes_plain() {
        assert_eq!(highlight("src/main.rs", &[row(RowKind::Note, "\\ No newline at end of file")], SYNTAX), vec![None]);
    }

    #[test]
    fn highlight_keeps_a_string_opened_on_the_old_side_out_of_the_new_side() {
        let alone = highlight("src/main.rs", &[row(RowKind::Added, "let x = 1;")], SYNTAX);
        let after_removal = highlight("src/main.rs", &[row(RowKind::Removed, "let s = \"open"), row(RowKind::Added, "let x = 1;")], SYNTAX);

        assert_eq!(colours_of(&after_removal[1]), colours_of(&alone[0]));
    }

    #[test]
    fn highlight_follows_the_syntax_theme_it_is_given() {
        let rows = [row(RowKind::Added, "fn main() {}")];

        let dark = highlight("src/main.rs", &rows, SYNTAX);
        let light = highlight("src/main.rs", &rows, EmbeddedThemeName::Github);

        assert_ne!(colours_of(&dark[0]), colours_of(&light[0]));
    }
}
