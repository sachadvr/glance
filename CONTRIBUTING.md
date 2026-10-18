# Contributing to glance

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

Open an issue with:
- the git command you ran;
- what you expected, and what you saw. A screenshot or the output piped to a file (`git log -p > out.txt`) helps a lot;
- your OS, your terminal, and the glance version (`cargo install --list`) or the commit you built from.

## Building and testing

You need Rust 1.89 or later and git.

```sh
cargo build
cargo clippy --all-targets   # must print no warning
cargo test                   # unit tests, git tests on throwaway repositories, app tests on a fake terminal
```

To try your build as git's pager without installing it:

```sh
git -c core.pager="$PWD/target/debug/glance" log -p
```

Or run `cargo run` inside any repository to open the branch browser.

## Making a change

Read [`ARCHITECTURE.md`](ARCHITECTURE.md) first: it is short. Most features are added rather than edited in:

| To add… | Start from |
|---|---|
| a screen or a tab | a new `View` in `src/app/views/` |
| a key or a clickable button | a `Binding` in the screen's `keymap()` (with a `hint` for the footer) |
| a format git can pipe | a detector in `src/input/` plus one line in `DETECTORS` |
| a theme | one entry in `THEMES` (`src/theme/catalog.rs`) |
| a git operation | a method on `Git` or `Repo` (`src/git/`) |

Step-by-step guides for each of these are in [`.claude/skills/`](.claude/skills). They are written for Claude Code, but read just as well by humans.

### Ground rules

- **Tests come with the change.**
  - Put unit tests next to the code.
  - Put app behaviour in `tests/app.rs`, using the helpers in `tests/support/`.
  - git tests create their own repository in the temp directory and never touch the network.
- **`domain/` stays pure.** No IO, no terminal, no git in it.
- **No `match` on concrete screens in the app.** Add a `View` method with a default instead.
- **Keys are declared once, with their footer hint.** No footer text is written by hand.
- **Comments explain *why*, not *what*.** Names are spelled out; guard clauses are preferred over nesting.
- **User-facing text** is French for now, until the interface is translated. Code, comments and commit messages are English.

### Pull requests

- Keep each pull request focused on one change.
- Describe what changed and how you checked it.
- CI runs clippy and the tests on Linux and macOS: both must pass.

By contributing, you agree that your contributions are licensed as described in the [README](README.md#license).
