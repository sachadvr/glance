# glance — architecture

A git pager (`core.pager = glance`) and a branch browser (`glance` run in a repository).

## Layers

```
main.rs ── picks the source: a pipe from git (pager) or the repository (browser)
runtime.rs ── terminal setup, graphics query (images), event loop, OSC 52 clipboard / OSC 22 pointer
app/ ── tabs of screens, keys and clicks → Commands → screens → Transitions/Effects
  views/ ── one file per screen, each an implementation of `View`
ui/ ── drawing primitives shared by screens (lists, stat rows, diff lines, clickable rows, images)
input/ ── recognising what git piped (log, oneline, reflog, stash list, branch list, status, blame, patch)
git/ ── running git (concrete `Git`, `Repo`, `Base`)
highlight/ ── syntax colours (syntect + two-face grammars)
theme/ ── colour themes, the saved choice
i18n/ ── what glance says, one `Strings` per language, picked from the locale
domain/ ── what is shown: files, commits, blame lines, diff rows, word emphasis (pure, no IO)
```

The language is chosen once at startup (`i18n::detect`) and, like the theme, handed to screens as a value
(`Context::strings`, `Paint::strings`, `View::keymap`): a text missing from a language does not compile.
How the terminal draws images (kitty, iTerm2, sixel or half blocks) is asked once by `runtime` and handed down the same
way (`App::set_picker`, `Context::picker`); the app starts with half blocks, which is what the tests draw.

Dependencies point downwards: `domain` knows nothing else; `input` and `git` produce `domain`
values; `app` composes everything; only `runtime` and `main` touch the terminal and the process.

## Flow of a key

1. `App::on_key` resolves the key through **layers** of bindings, first match wins:
   search (n/N/Esc while a search is confirmed) → the top screen's `keymap()` → the root layer
   (tab keys, base 1/2/3, reload, fetch; only on a tab's first screen) → global (/, y, t, q).
2. The resulting `Command` is either handled by the app (theme, search, tabs, base, fetch) or
   handed to the top `View::handle`, which returns a `Transition` (`Stay`, `Push`, `Pop`,
   `Return(index)`, `Quit`) and may push `Effect`s (`Error`, `Copy`, `SetBase`).
3. Clicks skip step 1: every clickable area drawn records a `Hit { area, command }`.

The footer is computed from the same layers (`keymap::hints`): a binding with a `hint` shows up,
bindings sharing a label are grouped ("n/p fichier").

## Data ownership

`Session` owns the data (files, commits, blame, base, fetch state). A tab's first screen reads it
live; a screen opened on top (a commit's files, a diff) keeps an `Rc` snapshot of the list it
browses, so a reload or a fetch never pulls an index from under it.

## Extension points

| To add…                         | Do                                                                                   |
|---------------------------------|--------------------------------------------------------------------------------------|
| a screen                        | a file in `app/views/` implementing `View`; push it from an existing screen          |
| a tab                           | a root `View` returning `tab_title()`, added to the tabs in `App::for_*`             |
| a key                           | a `Binding` in the screen's `keymap()` (with a `hint` to show it in the footer)      |
| an input format git can pipe    | a file in `input/` with `detect`, one line in `DETECTORS` (order matters, it is tested) |
| a theme                         | one entry in `theme::THEMES`                                                          |
| a language                      | a file in `i18n/` with a `Strings` value, one entry in `i18n::LANGUAGES`              |
| a git operation                 | a method on `Git`/`Repo`, parsing its output with `input::*`                           |

## Testing

- `domain`, `input`, `search`, `ui`, `theme`, `highlight`, `terminal`: unit tests next to the code.
- `git`: tests against a throwaway repository created with `git init` in the temp directory.
- `tests/`: the app driven through its public API on a `TestBackend` (keys, clicks, rendering).
