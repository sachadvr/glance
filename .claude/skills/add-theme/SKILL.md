---
name: add-theme
description: Add a colour theme to glance (e.g. Solarized, One Dark, a light variant). Use when the user asks for a new theme or palette.
---

# Add a theme

`THEMES` in `src/theme/catalog.rs` is the registry: a theme is one entry. The `t` key cycles through it in order
and the choice is saved by name in `$XDG_CONFIG_HOME/glance/theme` (or `~/.config/glance/theme`).

## Steps

1. Pick the syntax theme from `two_face::theme::EmbeddedThemeName` closest to the palette.
2. Add a `Theme { .. }` entry to `THEMES` (bump the array length). Fill every field with `hex(0x……)`:
   - `canvas: None` for dark themes drawn on the terminal's background; `Some((background, foreground))` for a
     light theme (it cannot rely on a dark terminal).
   - `added_bg`/`removed_bg` must stay readable under syntax colours; `*_emphasis_bg` is a stronger tint of the
     same hue (changed words); `selected_bg` and `hover_bg` must differ from each other and from the canvas.
3. The `name` is user-facing and saved to disk: unique, and never rename an existing theme (users' saved choice
   would silently fall back to the first theme).

## Tests (required)

- `every_theme_has_a_unique_name_and_an_existing_syntax_theme` covers the entry; run `cargo test --lib theme`.
- Look at it: `git diff | glance` (or `git log -p`) and press `t` until it shows, in both a dark and a light
  terminal if it is a light theme.
