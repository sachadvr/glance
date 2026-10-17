---
name: add-git-operation
description: Make glance run a new git command (read data like branches or file history, or an action like stage/checkout). Use when a feature needs information or an action git provides.
---

# Add a git operation

`src/git/` is concrete on purpose (no trait): `Git` runs git in one directory, `Repo` adds what only a
repository browser has (branch, main line, `Base`). Running git and parsing its output are separate.

## Steps

1. **Run** through `Git::run(&[..])` in `src/git/command.rs` (`Git::command` when the exit code must be read
   yourself, as `git diff --no-index` in `repo.rs`): it adds `-C root -c core.quotePath=false` and turns stderr
   into a `GitError`. Always pass `--no-color --no-ext-diff` to commands that print patches, and
   `-z` when reading paths. Never build a shell string.
2. **Parse** with `input::*` when the output is a format the pager already reads (`parse_patch`, `parse_log`);
   otherwise write a pure parser next to it and unit-test it on fixtures.
3. **Expose** a method returning `GitResult<domain type>` on `Git` (works in pager mode too, `Git::here()`) or on
   `Repo` (browser only). Error messages shown to the user are French.
4. **Call it from the app**: through a `Session` method if the result is shared state (it owns the data), or
   directly in a screen's `handle` via `context.session.git()`; report failures with `context.fail(error)`.
5. **Long or networked** operations (like `fetch`) run on a thread: follow `App::start_fetch` /
   `App::poll_background` (send only `Send` data such as `bool` or `Vec`, never an `Rc`). Mutating operations
   (stage, checkout, commit) must be explicit user actions and reload the session afterwards.

## Tests (required)

- In `src/git/`: a test on a throwaway repository (`git init` in the temp dir, as the existing tests do) —
  never the network, never the user's repository.
- Parser unit tests on real output fixtures.
