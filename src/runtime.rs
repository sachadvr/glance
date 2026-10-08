//! The terminal side: setting it up, the event loop, and giving it back as it was.

use std::error::Error;
use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::DefaultTerminal;

use crate::app::App;
use crate::terminal::{copy_to_clipboard, set_pointer};

// How often a background job (the fetch) is checked for while no key comes.
const BACKGROUND_TICK: Duration = Duration::from_millis(200);

pub fn run(mut app: App) -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;
    let outcome = event_loop(&mut terminal, &mut app);
    set_pointer(false)?;
    execute!(io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    outcome
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<(), Box<dyn Error>> {
    let mut is_pointing = false;
    while !app.should_quit() {
        terminal.draw(|frame| app.draw(frame))?;
        if app.hovered().is_some() != is_pointing {
            is_pointing = !is_pointing;
            set_pointer(is_pointing)?;
        }
        app.poll_background();
        if !event::poll(BACKGROUND_TICK)? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
            Event::Mouse(mouse) => app.on_mouse(mouse),
            _ => {}
        }
        if let Some(text) = app.take_clipboard() {
            copy_to_clipboard(&text)?;
        }
    }
    Ok(())
}
