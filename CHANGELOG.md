# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **English interface**, alongside French. The language follows the locale (`LC_ALL`, `LC_MESSAGES`, `LANG`), and `GLANCE_LANG` overrides it.
- **`git branch` in the pager** (`-v`, `-vv`, `-a`, `-r`, `--merged`…): a Branches tab, the current branch marked `*`; a branch opens on its tip commit's files.
- **`git status` in the pager**, long and short (`-s`, `-sb`): the files listed, each opening on its uncommitted diff, loaded from git where it ran.

### Changed
- Git errors are worded in the interface's language.

## [0.1.0] - 2026-10-18

First public release.

### Added
- **Pager mode:** git's diffs, logs (medium, oneline, graph), reflogs, stash lists and blames shown as navigable lists; anything else is passed to `less`.
- **Branch browser:** run `glance` in a repository to see uncommitted work and the branch's commits.
- **Base selection:** count changes from uncommitted work, the main branch or a chosen commit.
- **Background fetch** of `origin` when the browser starts.
- **Diffs:** syntax highlighting, line numbers on both sides, word-level emphasis of changed words, optional line wrapping.
- **Grouping:** files grouped by directory, with tests in their own section.
- **Blame:** jump from a line to the commit that wrote it.
- **Search** with `/`, `n` and `N`.
- **Copy** to the clipboard through OSC 52.
- **Mouse:** clicks, hover and wheel.
- **6 colour themes**; the choice is remembered.
