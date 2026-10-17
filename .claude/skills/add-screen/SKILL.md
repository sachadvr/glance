---
name: add-screen
description: Add a new screen (View) or a new tab to glance — e.g. a branch list, a file history, a stash detail. Use when the user asks for a new view, panel, list, tab or page in glance.
---

# Add a screen

A screen is one implementation of `View` (`src/app/view.rs`). The app only knows the trait: adding a screen
must not add a `match` on screen types anywhere.

## Steps

1. **Data first.** If the screen shows something new, add the type to `src/domain/` (pure) and how it is
   obtained to `src/git/` (see `add-git-operation`) or `src/input/` (see `add-input-format`).
2. **Create `src/app/views/<name>.rs`** and re-export it in `src/app/views/mod.rs`. Start from the closest
   existing screen:
   - a list read live from `Session` → `files.rs` / `commits.rs` (root screens);
   - a list of a snapshot it was opened with → `commit_files.rs` (holds an `Rc`/owned copy, never an index into `Session`);
   - scrolled text → `diff.rs`.
3. **Implement `View`** — only what the screen needs, the rest has defaults:
   - `keymap()`: `keymap::list_motions()` + `keymap::open("<label>")` + `keymap::back()` (non-root) + own bindings with `.hint(..)`.
   - `handle()`: `Command::Move(motion)` → `self.cursor.apply(motion, len)`; open → `Transition::Push(Box::new(..))`;
     back → `Transition::Pop` or `Transition::Return(index)` to put the parent's cursor on an item.
     Errors: `context.fail(error)`. Never panic on an empty list (`Command::Open if length > 0`).
   - `render()`: call `self.cursor.drawn(list_area)` first (page size), draw with `crate::ui` primitives
     (`draw_list`, `stat_line`, `diff_lines`, `Row::push_target` for clickable spans), return the `Hit`s
     (+ `back_hit(title)` if it has a "←" title).
   - `search_texts` / `position` / `select` to make `/` and n/N work; `yank` for `y` (and `Y` with `whole`).
   - `prepare()` for lazy loading before a draw, `restyle()` if it caches theme-dependent colours.
   - Root screens only: `tab_title()` (key, label, count, stats), optional `tools()` and `shows_base()`.
4. **Reach it**: push it from an existing screen's `handle`, or, for a tab, add it to the root list in
   `App::for_repo` / `App::for_document` (`src/app/mod.rs`). Tab keys and hints come from `tab_title()`.
5. **Check key collisions**: the screen's chords must not overlap `keymap::global()` (nor the root layer for a
   root screen) — add the screen to `tests/keymap.rs`.

## Tests (required)

- In `tests/app.rs`: open the screen from the app with the `tests/support` helpers, assert what `frame(&mut app)`
  shows, the footer hints, back navigation, search, yank, a click on a row, an empty state, and a tiny
  terminal (`render(&mut app, 10, 5)`, `render(&mut app, 1, 1)`) not panicking.
- Run the `ship` skill's checks.
