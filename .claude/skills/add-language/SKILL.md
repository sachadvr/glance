---
name: add-language
description: Translate glance into a new language, or add a new user-facing text to every language. Use when the user asks for a translation, a locale, or when a change shows new text on screen.
---

# Add a language, or a text

Everything glance says is a field of `Strings` (`src/i18n/mod.rs`). A language is one `Strings` value in its own
file; it is a struct literal, so a language missing a text does not compile.

## A new language

1. Copy `src/i18n/en.rs` to `src/i18n/<code>.rs` (ISO 639-1 code: `de`, `es`, `pt`…) and rename the constant
   (`GERMAN`). Translate every field:
   - keep the symbols (`⏎`, `▾`, `◀`, `▶`, `✔`, `✘`, `⟳`, `·`) and the `{…}` placeholders;
   - plurals are functions: agree them the way the language does (French counts 0 as singular, English as plural);
   - hints are short (the footer is one line): prefer a verb or a noun of one word.
2. Declare it in `src/i18n/mod.rs`: `mod <code>;`, `pub use <code>::GERMAN;`, and add it to `LANGUAGES`
   (bump the array length). The first entry stays the fallback (English).
3. `detect` needs nothing: a locale starting with the code (`de_DE.UTF-8`) now picks it; `GLANCE_LANG=<code>` forces it.
4. Mention it in README.md (Features, Configuration) and CHANGELOG.md.

## A new text

1. Add a field to `Strings` with a doc comment when the context is not obvious (a function `fn(usize) -> String` when it
   holds a number or a name).
2. Fill it in every file of `src/i18n/` — the compiler lists the ones missing it.
3. Read it where it is shown: `context.strings` in `handle`, `paint.strings` in `render`, the `strings` given to `keymap()`,
   `tab_title()`, `tools()`; `self.strings` in `App`.

## Tests (required)

- `cargo test --lib i18n` (unique codes, detection, plurals): add a plural/detection case for a new language.
- `tests/language.rs`: `every_language_draws_every_screen` covers a new language automatically; for a new language,
  add a test asserting a few of its texts on screen, like the English ones.
