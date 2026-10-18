//! Shared harness for the integration tests: realistic git output, an app built from it, keys and clicks,
//! and the rendered screen read back as text.
#![allow(dead_code)]

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use glance::i18n::FRENCH;
use glance::app::App;
use glance::git::Repo;
use image::{ImageFormat, Rgb, RgbImage};
use glance::input::{self, Document};
use glance::theme::ThemeSelection;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::text::Span;
use ratatui::Terminal;

/// `git diff`: three source files and one test, sorted by the app to src/alpha, src/beta, src/gamma, tests/alpha_test.
/// alpha and beta carry a line far wider than 80 columns; gamma is 61 rows long (hunk + 60 lines).
pub fn diff() -> String {
    let long = "wrapword ".repeat(20);
    let mut text = format!(
        "diff --git a/tests/alpha_test.rs b/tests/alpha_test.rs
index 1111111..2222222 100644
--- a/tests/alpha_test.rs
+++ b/tests/alpha_test.rs
@@ -1 +1 @@
-assert!(false);
+assert!(true);
diff --git a/src/alpha.rs b/src/alpha.rs
index 1111111..2222222 100644
--- a/src/alpha.rs
+++ b/src/alpha.rs
@@ -1,3 +1,3 @@
 fn alpha() {{
-    old_alpha();
+    new_alpha(); // {long}
 }}
diff --git a/src/beta.rs b/src/beta.rs
index 1111111..2222222 100644
--- a/src/beta.rs
+++ b/src/beta.rs
@@ -1 +1 @@
-let beta = 1;
+let beta = 2; // {long}
diff --git a/src/gamma.rs b/src/gamma.rs
new file mode 100644
--- /dev/null
+++ b/src/gamma.rs
@@ -0,0 +1,60 @@
"
    );
    for line in 1..=60 {
        text.push_str(&format!("+gamma line {line:02}\n"));
    }
    text
}

/// `git log -p`: two commits with their patches.
pub const LOG: &str = "commit 1111111aaaabbbbccccdddd (HEAD -> main)
Author: Grace H <grace@example.com>
Date:   2 days ago

    Fix the feed

diff --git a/src/feed.rs b/src/feed.rs
index 1111111..2222222 100644
--- a/src/feed.rs
+++ b/src/feed.rs
@@ -1 +1 @@
-old feed
+new feed
diff --git a/src/parser.rs b/src/parser.rs
index 1111111..2222222 100644
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1 +1,2 @@
 parse
+parse more
commit 2222222bbbbccccdddd
Author: Lea <lea@example.com>
Date:   3 days ago

    Add the parser

diff --git a/src/parser.rs b/src/parser.rs
new file mode 100644
--- /dev/null
+++ b/src/parser.rs
@@ -0,0 +1 @@
+parse
diff --git a/tests/parser_test.rs b/tests/parser_test.rs
new file mode 100644
--- /dev/null
+++ b/tests/parser_test.rs
@@ -0,0 +1 @@
+assert!(parse());
";

/// `git show`: one commit with its patch.
pub const SHOW: &str = "commit 1111111aaaabbbbccccdddd (HEAD -> main)
Author: Grace H <grace@example.com>
Date:   2 days ago

    Fix the feed

diff --git a/src/feed.rs b/src/feed.rs
index 1111111..2222222 100644
--- a/src/feed.rs
+++ b/src/feed.rs
@@ -1 +1 @@
-old feed
+new feed
diff --git a/src/parser.rs b/src/parser.rs
index 1111111..2222222 100644
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1 +1,2 @@
 parse
+parse more
";

/// `git log` without patches: the files of a commit must be asked of git.
pub const LOG_NO_PATCH: &str = "commit 0123456789abcdef (HEAD -> main)
Author: Grace H <grace@example.com>
Date:   2 days ago

    Fix the feed

commit fedcba9876543210
Author: Lea <lea@example.com>
Date:   3 days ago

    Add the parser
";

/// `git log --oneline`.
pub const ONELINE: &str = "0123456 (HEAD -> main) First\n89abcde Second\nfedcba9 Third\n";

/// `git reflog`.
pub const REFLOG: &str = "238d6155 (HEAD -> hotfix) HEAD@{0}: cherry-pick: fix(parser): Stop dropping regions\n26ce5db4 HEAD@{1}: commit: Init hotfix\n238d6155 HEAD@{2}: checkout: moving from master to hotfix\n";

/// `git stash list`.
pub const STASH: &str = "stash@{0}: WIP on main: 238d615 Fix the feed\nstash@{1}: On feature-x: promotion of attributes\n";

/// `git branch -vv`: the current branch, one with its upstream, a multibyte name.
pub const BRANCHES: &str = "  feature/search 89abcde [origin/feature/search: ahead 1] Add the search\n* main           0123456 Fix the feed\n  été            fedcba9 Corrige l'écran\n";

/// `git branch` on a detached HEAD: `git show HEAD` opens it in any repository the tests run in.
pub const DETACHED_BRANCHES: &str = "* (HEAD detached at 0123456)\n  main\n";

/// `git status`, with paths no repository the tests run in holds: they stay as listed, without a patch.
pub const STATUS: &str = "On branch main
Changes to be committed:
  (use \"git restore --staged <file>...\" to unstage)
\tmodified:   glance-fixture/app/feed.rs
\trenamed:    glance-fixture/docs/old.md -> glance-fixture/docs/new.md

Changes not staged for commit:
  (use \"git add <file>...\" to update what will be committed)
\tdeleted:    glance-fixture/app/parser.rs

Untracked files:
  (use \"git add <file>...\" to include in what will be committed)
\tglance-fixture/tests/feed_test.rs

";

/// `git status -sb`, the same files.
pub const STATUS_SHORT: &str = "## main...origin/main [ahead 1]\nM  glance-fixture/app/feed.rs\nR  glance-fixture/docs/old.md -> glance-fixture/docs/new.md\n D glance-fixture/app/parser.rs\n?? glance-fixture/tests/feed_test.rs\n";

/// `git blame`: two committed lines, then one not committed yet.
pub const BLAME: &str = "7bfd4f45 (Linus 2024-08-21 23:03:02 +0200 1) <?php\n238d6155 (Grace Hopper 2026-10-16 18:01:20 +0200 2) return $feed;\n00000000 (Not Committed Yet 2026-10-18 10:00:00 +0200 3) echo 'new';\n";

pub fn document(text: &str) -> Document {
    input::read(text).expect("git output the app recognises")
}

pub fn app(text: &str) -> App {
    App::for_document(document(text), None, ThemeSelection::default(), &FRENCH)
}

pub fn blame_app() -> App {
    App::for_document(document(BLAME), Some("src/feed.php".to_string()), ThemeSelection::default(), &FRENCH)
}

// ---------------------------------------------------------------- repository

/// A throwaway repository in the temp directory, "main" with a first commit: what the browser opens. Gone once dropped.
pub struct TempRepo {
    pub root: PathBuf,
}

impl TempRepo {
    pub fn new(name: &str) -> TempRepo {
        let root = std::env::temp_dir().join(format!("glance-app-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a temp directory");
        let repo = TempRepo { root };
        repo.git(&["init", "--quiet", "--initial-branch=main"]);
        repo.write("README.md", b"hello\n");
        repo.commit("README.md", "Start");
        repo
    }

    // The user's own config (signing, hooks) must not leak into the test.
    pub fn git(&self, args: &[&str]) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null"])
            .args(args)
            .output()
            .expect("git runs");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    pub fn write(&self, path: &str, content: &[u8]) {
        fs::write(self.root.join(path), content).expect("a file written");
    }

    pub fn commit(&self, path: &str, subject: &str) {
        self.git(&["add", path]);
        self.git(&["commit", "--quiet", "-m", subject]);
    }

    pub fn app(&self) -> App {
        App::for_repo(Repo::open(&self.root).expect("the repository opens"), ThemeSelection::default(), &FRENCH)
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A PNG made here rather than kept as a binary fixture. It darkens downwards: a cell of one colour would be drawn as a
/// blank, a half block "▀" says the image is there.
pub fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    RgbImage::from_fn(width, height, |_, y| Rgb([255 - (255 * y / height) as u8, 0, 0])).write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).expect("a PNG encoded");
    bytes
}

/// What says it is a PNG and is not one past its signature.
pub const BROKEN_PNG: &[u8] = b"\x89PNG\r\n\x1a\nnot an image";

// ---------------------------------------------------------------- input

pub fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::from(code));
}

pub fn ctrl(app: &mut App, character: char) {
    app.on_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL));
}

/// Each character as its own key press: `keys(app, "/alpha")`.
pub fn keys(app: &mut App, text: &str) {
    for character in text.chars() {
        let modifiers = if character.is_ascii_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
        app.on_key(KeyEvent::new(KeyCode::Char(character), modifiers));
    }
}

pub fn search(app: &mut App, query: &str) {
    keys(app, &format!("/{query}"));
    key(app, KeyCode::Enter);
}

fn mouse(app: &mut App, kind: MouseEventKind, (column, row): (u16, u16)) {
    app.on_mouse(MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE });
}

pub fn click(app: &mut App, at: (u16, u16)) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), at);
}

pub fn hover(app: &mut App, at: (u16, u16)) {
    mouse(app, MouseEventKind::Moved, at);
}

pub fn wheel_down(app: &mut App) {
    mouse(app, MouseEventKind::ScrollDown, (0, 0));
}

pub fn wheel_up(app: &mut App) {
    mouse(app, MouseEventKind::ScrollUp, (0, 0));
}

/// What `y` copies, the cheapest way to read where a list cursor is.
pub fn yank(app: &mut App) -> Option<String> {
    keys(app, "y");
    app.take_clipboard()
}

// ---------------------------------------------------------------- output

/// A drawn frame: per row, each visible cell's column and symbol (the cells hidden under a wide character skipped).
pub struct Screen {
    pub rows: Vec<Vec<(u16, String)>>,
    pub width: u16,
    pub height: u16,
}

pub fn render(app: &mut App, width: u16, height: u16) -> Screen {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal.draw(|frame| app.draw(frame)).expect("drawn");
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let mut rows = Vec::new();
    for y in 0..area.height {
        let mut row = Vec::new();
        let mut x = 0;
        while x < area.width {
            let symbol = buffer[(x, y)].symbol().to_string();
            let width = Span::raw(symbol.as_str()).width().max(1) as u16;
            row.push((x, symbol));
            x += width;
        }
        rows.push(row);
    }
    Screen { rows, width: area.width, height: area.height }
}

/// The standard frame most tests look at.
pub fn frame(app: &mut App) -> Screen {
    render(app, 80, 20)
}

impl Screen {
    pub fn line(&self, y: u16) -> String {
        self.rows[usize::from(y)].iter().map(|(_, symbol)| symbol.as_str()).collect::<String>().trim_end().to_string()
    }

    pub fn lines(&self) -> Vec<String> {
        (0..self.height).map(|y| self.line(y)).collect()
    }

    pub fn text(&self) -> String {
        self.lines().join("\n")
    }

    pub fn header(&self) -> String {
        self.line(0)
    }

    pub fn tab_row(&self) -> String {
        self.line(1)
    }

    pub fn footer(&self) -> String {
        self.line(self.height - 1).trim().to_string()
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.lines().iter().any(|line| line.contains(needle))
    }

    /// Rows (in the body, above the footer) whose text contains `needle`.
    pub fn rows_containing(&self, needle: &str) -> Vec<u16> {
        (0..self.height.saturating_sub(1)).filter(|&y| self.line(y).contains(needle)).collect()
    }

    /// The cell where `needle` starts, top to bottom.
    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        for (y, row) in self.rows.iter().enumerate() {
            let mut text = String::new();
            let mut columns = Vec::new();
            for (x, symbol) in row {
                columns.push((text.len(), *x));
                text.push_str(symbol);
            }
            if let Some(offset) = text.find(needle) {
                let x = columns.iter().rev().find(|(start, _)| *start <= offset).map_or(0, |(_, x)| *x);
                return Some((x, y as u16));
            }
        }
        None
    }

    /// The row the list cursor sits on: a list marks it with "›" in its first column.
    pub fn selected(&self) -> Option<String> {
        self.lines().into_iter().find(|line| line.starts_with('›'))
    }
}
