---
name: add-key
description: Add or change a keyboard shortcut or a clickable action in glance (footer hint included). Use when the user asks for a new key, shortcut, binding, button or mouse action.
---

# Add a key or a click

Keys are data: a `Binding { chords, command, hint }` (`src/app/keymap.rs`). The footer is computed from the
bindings in force, so a key and its help can never disagree.

## Steps

1. **Command.** Reuse a `Command` (`src/app/command.rs`) if one fits; otherwise add a variant. A new motion is a
   `Motion` variant with its rule in `Motion::apply` (+ a case in its test).
2. **Pick the layer** (first match wins, in this order, see `App::layers`):
   - search layer (n/N/Esc while a search is confirmed) — leave alone;
   - the screen's `keymap()` — for what one screen does;
   - the root layer `App::root_bindings` — tab switching and repository actions, only on a tab's first screen;
   - `keymap::global()` — what every screen does (/, y, t, q).
3. **Declare the binding**: `Binding::new(command, &[char('x'), key(KeyCode::F(2)), ctrl('x')])`; add
   `.hint("x", "libellé")` to show it in the footer. Bindings sharing a label are grouped ("n/p fichier"), so give
   paired keys the same label. Uppercase letters already imply Shift; Control is explicit (`ctrl`).
4. **Handle the command**: in the screen's `View::handle`, or in `App::execute` if the app owns it (theme,
   search, tabs, base, fetch). User-facing text in French.
5. **A clickable span** for the same action: `row.push_target(span, Command::…)` in the screen's render or in
   `src/app/chrome.rs`. Never map a click to a fake key press. Hover styling is automatic.

## Tests (required)

- `tests/keymap.rs` must stay green (no chord bound twice across layers); add the screen there if new.
- `tests/app.rs`: press the key (`keys(&mut app, "x")`) and assert the effect; assert the footer shows the hint
  (`frame(&mut app).footer()`); click the span if there is one (`click(&mut app, screen.find("…").unwrap())`).
