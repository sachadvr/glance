# glance

Rust TUI (ratatui 0.30 + crossterm 0.29, syntect via two-face): a git pager (`core.pager`, `pager.diff/log/show/blame`
can point to `glance`) and a branch browser when run in a repository. Read `ARCHITECTURE.md` first.

## Toolchain

Rust 1.90+ (edition 2024) and git. Machine-specific notes (toolchain paths, local install) belong in
`CLAUDE.local.md`, which is git-ignored.

- Check: `cargo clippy --all-targets` (must be silent) and `cargo test` (all green, no `#[ignore]` without a `BUG:` reason).
- Install for git to use: `cargo install --path . --force` (release profile is unstripped on purpose, see Cargo.toml).

## Extending — add, don't edit

Each extension point has a project skill; use it instead of improvising:

| Change                          | Skill               |
|---------------------------------|---------------------|
| a screen / a tab                | `add-screen`        |
| a key or a mouse action         | `add-key`           |
| a format git pipes to the pager | `add-input-format`  |
| a theme                         | `add-theme`         |
| a language, or a new text       | `add-language`      |
| a git operation                 | `add-git-operation` |
| verify, then install the binary | `ship`              |

## Rules the design depends on

- `domain/` stays pure: no IO, no ratatui, no git. `input/` and `git/` produce domain values; only `runtime.rs`
  and `main.rs` touch the terminal or the process.
- The app never matches on concrete screens: behaviour lives in `View` implementations (`app/views/`).
  If you are about to add a `match` on a screen type in `app/mod.rs` or `app/chrome.rs`, add a `View` method
  with a default instead.
- Keys are declared once, in a keymap, with their footer `hint`. Never hard-code key handling or footer text.
- Clicks carry a `Command` (`Row::push_target`, `Hit`); never simulate a key press.
- The theme is a value (`&Theme`) passed down; there is no global theme. Colours computed from a theme are
  recomputed in `View::restyle`.
- A screen opened on top of another keeps an `Rc` snapshot of what it browses; only a tab's root screen reads
  `Session` live, and clamps its cursor with `ListCursor::within`.
- Opening/leaving a screen goes through `Transition` (`Push`, `Pop`, `Return(index)`); a search ends when the
  screen or its content changes (`Effect::ContentChanged`).
- No trait with a single implementation, no speculative layer (see the `Git`, `highlight`, `theme` modules).

## Style

- Comments say *why*, in the terse voice already used (`// A blame does not name its file, but git …`); never *what*.
- Descriptive names, no abbreviations; guard clauses (`let … else`) over nesting; one space around operators,
  no vertical alignment; long single-line expressions are fine (rustfmt is not used).
- User-facing text never sits inline: it is a field of `i18n::Strings`, filled in every language (`src/i18n/*.rs`), reached
  through `context.strings` / `paint.strings` / the `strings` given to `keymap()`. Git errors are `GitError` values worded by
  `Strings::git_error`. Code, comments and commits are English.

## Tests

- Unit tests sit next to the code (`#[cfg(test)] mod tests`), named as sentences
  (`emphasis_skips_the_shared_prefix_and_suffix`).
- `git/` tests build a throwaway repository with `git init` in the temp directory — never the network.
- App behaviour: `tests/app.rs` through `tests/support/mod.rs` (`app(text)`, `keys`, `key`, `click`, `hover`,
  `wheel_down`, `search`, `yank`, `frame(app)` → `Screen` with `footer()`, `find()`, `selected()`…) on a `TestBackend`.
- `tests/keymap.rs` checks that no chord is bound twice across layers: extend it when adding a screen.
- Anything that writes `~/.config` (theme cycling) must point `XDG_CONFIG_HOME` to a temp dir.
- Every feature or fix comes with tests; a failing behaviour found but not fixed is a test marked `#[ignore = "BUG: …"]`.
