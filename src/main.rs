use std::error::Error;
use std::io::{self, IsTerminal, Read, Write};
use std::process::{Command, ExitCode, Stdio};

use glance::app::App;
use glance::git::Repo;
use glance::i18n::{self, Strings};
use glance::input;
use glance::runtime;
use glance::theme::ThemeSelection;

// `git log -p` over a whole history never ends for a pager; past this, what was read is what is shown.
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("glance: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    if io::stdin().is_terminal() {
        return runtime::run(App::for_repo(Repo::discover()?, ThemeSelection::restore(), language()));
    }
    let mut piped = Vec::new();
    io::stdin().take(MAX_INPUT_BYTES).read_to_end(&mut piped)?;
    if !io::stdout().is_terminal() {
        io::stdout().write_all(&piped)?;
        return Ok(());
    }

    match input::read(&String::from_utf8_lossy(&piped)) {
        Some(document) => runtime::run(App::for_document(document, blamed_path(), ThemeSelection::restore(), language())),
        None => page_plainly(&piped),
    }
}

fn language() -> &'static Strings {
    i18n::detect(|name| std::env::var(name).ok())
}

// A blame does not name its file, but git, which runs glance as its pager, was given it last on its command line
// ("git blame -L 10,20 src/Kernel.php"). Only the colouring depends on it: a miss leaves the code plain.
fn blamed_path() -> Option<String> {
    let output = Command::new("ps").args(["-o", "args=", "-p", &std::os::unix::process::parent_id().to_string()]).output().ok()?;
    let command = String::from_utf8_lossy(&output.stdout).into_owned();
    let last = command.split_whitespace().last()?;

    (!last.starts_with('-') && last != "blame").then(|| last.to_string())
}

// What glance does not read (`git diff --stat`, `git shortlog`) goes to less, as git would have sent it.
fn page_plainly(input: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut less = Command::new("less").arg("-FRX").stdin(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = less.stdin.take() {
        // Quitting less before the end closes the pipe: not an error.
        let _ = stdin.write_all(input);
    }
    less.wait()?;
    Ok(())
}
