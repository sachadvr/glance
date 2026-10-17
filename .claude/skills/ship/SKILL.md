---
name: ship
description: Verify glance (clippy, tests, pager smoke test) and install the binary git uses. Use after any change to glance, or when the user asks to build, test, install, or "make git use it".
---

# Ship

Run from the repository root (see `CLAUDE.local.md` for machine-specific toolchain notes, if any).

1. **Lint**: `cargo clippy --all-targets -- -D warnings` → must succeed with no warning.
2. **Test**: `cargo test` → every suite `ok`, 0 failed.
   An `ignored` count is only acceptable for tests marked `#[ignore = "BUG: …"]` — report them.
3. **Pager smoke test** (no terminal needed — output not a TTY must pass through unchanged):
   `printf 'diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b\n' | cargo run --quiet | head -2`
4. **Install** (only when the user wants git to use the new build):
   `cp ~/.cargo/bin/glance ~/.cargo/bin/glance.old; cargo install --path . --force --quiet`.
   If git is set up with `core.pager` / `pager.<command>` pointing to `glance`, nothing else changes; do not edit
   the user's git config unasked. Roll back with `mv ~/.cargo/bin/glance.old ~/.cargo/bin/glance`.
5. **Report** what could not be verified: the interactive UI is not exercised by these steps beyond
   `tests/app.rs` — ask the user to try `git log -p` / `git diff` in a real terminal for visual changes.

Never commit unless asked; never add a `Co-Authored-By` trailer.
