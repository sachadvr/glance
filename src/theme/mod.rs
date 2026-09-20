//! The colour themes and which one is in use.

mod catalog;
mod saved;

pub use catalog::{Theme, THEMES};

/// Which theme is in use: restored from the last session, advanced by the theme key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeSelection {
    index: usize,
}

impl ThemeSelection {
    /// The theme picked last time, else the first one.
    pub fn restore() -> ThemeSelection {
        ThemeSelection::named(saved::load().as_deref())
    }

    pub fn current(self) -> &'static Theme {
        &THEMES[self.index]
    }

    /// Switches to the next theme, wrapping around, and remembers it for the next session.
    pub fn cycle(&mut self) {
        *self = self.next();
        saved::save(self.current().name);
    }

    // An unknown name (a theme renamed or removed since) falls back to the first theme.
    fn named(name: Option<&str>) -> ThemeSelection {
        let index = name.and_then(|name| THEMES.iter().position(|theme| theme.name == name.trim()));
        ThemeSelection { index: index.unwrap_or(0) }
    }

    fn next(self) -> ThemeSelection {
        ThemeSelection { index: (self.index + 1) % THEMES.len() }
    }
}

impl Default for ThemeSelection {
    /// The first theme, without reading the saved choice.
    fn default() -> ThemeSelection {
        ThemeSelection { index: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_selects_the_first_theme() {
        assert_eq!(ThemeSelection::default().current().name, THEMES[0].name);
    }

    #[test]
    fn named_finds_a_saved_name_despite_a_trailing_newline() {
        assert_eq!(ThemeSelection::named(Some("Nord\n")).current().name, "Nord");
    }

    #[test]
    fn named_falls_back_to_the_first_theme_for_an_unknown_or_missing_name() {
        assert_eq!(ThemeSelection::named(Some("Solarized")), ThemeSelection::default());
        assert_eq!(ThemeSelection::named(None), ThemeSelection::default());
    }

    #[test]
    fn next_advances_and_wraps_after_the_last_theme() {
        assert_eq!(ThemeSelection::default().next().current().name, THEMES[1].name);
        assert_eq!(ThemeSelection { index: THEMES.len() - 1 }.next(), ThemeSelection::default());
    }
}
