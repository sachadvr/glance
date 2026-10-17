---
name: add-input-format
description: Teach glance's pager to recognise a new kind of git output piped to it (e.g. git range-diff, git shortlog, git log --format=fuller, git worktree list). Use when a git command paged through glance falls back to less or is misread.
---

# Add an input format

`input::read` strips colours, then tries `DETECTORS` (`src/input/mod.rs`) in order; the first that recognises
the text wins. A format is one file + one registry line.

## Steps

1. **Collect real output** of the git command (with and without colours, with `--graph` if it applies) and turn
   it into test fixtures. Decide which `Document` it becomes (`Diff`, `Log`, `Blame`). If none fits, add a
   variant — then `App::for_document` needs a root screen for it (see `add-screen`).
2. **Create `src/input/<format>.rs`** modelled on `stash.rs`:
   ```rust
   pub(super) fn detect(lines: &[&str]) -> Option<Document> { parse_x(lines).map(Document::Log) }
   ```
   The parser must refuse anything that is not certainly its format (return `None` on the first unknown line,
   as `parse_stash_list` does): a lenient detector steals other formats.
   Build domain values with their constructors (`Commit::new(Revision::Hash(..), subject)`, `FileDiff::new(path)`);
   leave `files: None` when the output has no patches (the app loads them through git).
3. **Register it** in `DETECTORS` at the right position and update the comment above it explaining the order.
   Reuse helpers from sibling modules (`oneline::is_reflog_selector`, `patch::file_start_path`) rather than copy.
4. If the header should name the command, adjust the label in `App::for_document`.

## Tests (required)

- Unit tests in the new file: a typical sample, decorations/graph lanes, multibyte text, and refusals.
- In `src/input/mod.rs` tests: `read` returns the new document, AND the existing formats are still recognised
  as before (order regressions show up here — the blame-before-oneline and log-before-patch tests must pass).
- In `tests/app.rs`: `app(FIXTURE)` renders it and opens an item.
