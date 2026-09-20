use ratatui::style::Color;
use two_face::theme::EmbeddedThemeName;

/// The interface's colours and the grammar theme the code is coloured with.
pub struct Theme {
    pub name: &'static str,
    pub syntax: EmbeddedThemeName,
    /// Painted under everything: a light theme cannot rely on a dark terminal's own background.
    pub canvas: Option<(Color, Color)>,
    pub accent: Color,
    pub on_accent: Color,
    pub blue: Color,
    pub green: Color,
    pub red: Color,
    pub cyan: Color,
    pub grey: Color,
    pub added_bg: Color,
    pub removed_bg: Color,
    pub added_emphasis_bg: Color,
    pub removed_emphasis_bg: Color,
    pub hunk_bg: Color,
    pub selected_bg: Color,
    pub hover_bg: Color,
}

const fn hex(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

// The registry: adding a theme is one more entry here, the theme key cycles through them in this order.
pub const THEMES: [Theme; 6] = [
    Theme {
        name: "Tokyo Night",
        syntax: EmbeddedThemeName::CatppuccinMocha,
        canvas: None,
        accent: hex(0xbb9af7),
        on_accent: hex(0x000000),
        blue: hex(0x7aa2f7),
        green: hex(0x9ece6a),
        red: hex(0xf7768e),
        cyan: hex(0x7dcfff),
        grey: hex(0x565f89),
        added_bg: hex(0x20303b),
        removed_bg: hex(0x37222c),
        added_emphasis_bg: hex(0x2d4f40),
        removed_emphasis_bg: hex(0x5c2b3b),
        hunk_bg: hex(0x1f2335),
        selected_bg: hex(0x2f334d),
        hover_bg: hex(0x292e42),
    },
    Theme {
        name: "Catppuccin Mocha",
        syntax: EmbeddedThemeName::CatppuccinMocha,
        canvas: None,
        accent: hex(0xcba6f7),
        on_accent: hex(0x1e1e2e),
        blue: hex(0x89b4fa),
        green: hex(0xa6e3a1),
        red: hex(0xf38ba8),
        cyan: hex(0x89dceb),
        grey: hex(0x6c7086),
        added_bg: hex(0x273b33),
        removed_bg: hex(0x3c2633),
        added_emphasis_bg: hex(0x2e5a43),
        removed_emphasis_bg: hex(0x5e2d40),
        hunk_bg: hex(0x262637),
        selected_bg: hex(0x313244),
        hover_bg: hex(0x2a2b3c),
    },
    Theme {
        name: "Dracula",
        syntax: EmbeddedThemeName::Dracula,
        canvas: None,
        accent: hex(0xbd93f9),
        on_accent: hex(0x282a36),
        blue: hex(0x8be9fd),
        green: hex(0x50fa7b),
        red: hex(0xff5555),
        cyan: hex(0xff79c6),
        grey: hex(0x6272a4),
        added_bg: hex(0x253a2e),
        removed_bg: hex(0x432a32),
        added_emphasis_bg: hex(0x2d5c3c),
        removed_emphasis_bg: hex(0x6b303f),
        hunk_bg: hex(0x2c2f3f),
        selected_bg: hex(0x44475a),
        hover_bg: hex(0x383a4a),
    },
    Theme {
        name: "Nord",
        syntax: EmbeddedThemeName::Nord,
        canvas: None,
        accent: hex(0xb48ead),
        on_accent: hex(0x2e3440),
        blue: hex(0x81a1c1),
        green: hex(0xa3be8c),
        red: hex(0xbf616a),
        cyan: hex(0x88c0d0),
        grey: hex(0x616e88),
        added_bg: hex(0x2f3d38),
        removed_bg: hex(0x3f2e33),
        added_emphasis_bg: hex(0x3c5e4e),
        removed_emphasis_bg: hex(0x66363f),
        hunk_bg: hex(0x343b4a),
        selected_bg: hex(0x434c5e),
        hover_bg: hex(0x3b4252),
    },
    Theme {
        name: "Gruvbox Dark",
        syntax: EmbeddedThemeName::GruvboxDark,
        canvas: None,
        accent: hex(0xd3869b),
        on_accent: hex(0x282828),
        blue: hex(0x83a598),
        green: hex(0xb8bb26),
        red: hex(0xfb4934),
        cyan: hex(0x8ec07c),
        grey: hex(0x928374),
        added_bg: hex(0x32361a),
        removed_bg: hex(0x3c1f1e),
        added_emphasis_bg: hex(0x4b5c22),
        removed_emphasis_bg: hex(0x66271f),
        hunk_bg: hex(0x32302f),
        selected_bg: hex(0x504945),
        hover_bg: hex(0x3c3836),
    },
    Theme {
        name: "GitHub clair",
        syntax: EmbeddedThemeName::Github,
        canvas: Some((hex(0xffffff), hex(0x1f2328))),
        accent: hex(0x8250df),
        on_accent: hex(0xffffff),
        blue: hex(0x0969da),
        green: hex(0x1a7f37),
        red: hex(0xcf222e),
        cyan: hex(0x1b7c83),
        grey: hex(0x6e7781),
        added_bg: hex(0xdafbe1),
        removed_bg: hex(0xffebe9),
        added_emphasis_bg: hex(0xaceebb),
        removed_emphasis_bg: hex(0xffcecb),
        hunk_bg: hex(0xddf4ff),
        selected_bg: hex(0xd8dee4),
        hover_bg: hex(0xeaeef2),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_a_unique_name_and_an_existing_syntax_theme() {
        let syntax_themes = two_face::theme::extra();
        for (index, theme) in THEMES.iter().enumerate() {
            syntax_themes.get(theme.syntax);
            assert!(THEMES[..index].iter().all(|earlier| earlier.name != theme.name), "{} is listed twice", theme.name);
        }
    }

    #[test]
    fn hex_splits_the_red_green_and_blue_bytes() {
        assert_eq!(hex(0x0969da), Color::Rgb(0x09, 0x69, 0xda));
    }
}
