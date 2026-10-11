//! Cycling themes. Its own test binary: `ThemeSelection::cycle` saves the choice under $XDG_CONFIG_HOME, which this
//! process points at a temporary directory so the user's ~/.config/glance/theme is never touched.

mod support;

use std::fs;

use glance::app::command::Command;
use glance::theme::THEMES;
use ratatui::crossterm::event::KeyCode;
use support::*;

#[test]
fn t_cycles_through_every_theme_on_every_screen_and_the_header_names_it() {
    let config = std::env::temp_dir().join(format!("glance-theme-test-{}", std::process::id()));
    // SAFETY: the only test of this binary: no other thread reads the environment meanwhile.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &config) };

    let mut files = app(&diff());
    let mut diff_screen = app(&diff());
    key(&mut diff_screen, KeyCode::Enter);
    keys(&mut diff_screen, "w");
    let mut commit = app(SHOW);
    let mut unfolded = app(LOG);
    keys(&mut unfolded, "s");
    let mut blame = blame_app();
    let mut screens = [&mut files, &mut diff_screen, &mut commit, &mut unfolded, &mut blame];

    for app in screens.iter_mut() {
        let before = frame(app).text();
        for step in 1..=THEMES.len() {
            keys(app, "t");
            let expected = THEMES[step % THEMES.len()].name;
            let shown = frame(app);
            assert!(shown.header().contains(&format!("[t] {expected}")), "{}", shown.header());
            assert_eq!(fs::read_to_string(config.join("glance").join("theme")).ok().as_deref(), Some(expected));
            render(app, 1, 1);
            render(app, 12, 5);
        }
        assert_eq!(frame(app).text(), before, "a full cycle comes back to the same screen");
        assert!(!app.should_quit());
    }

    let switch = frame(&mut files).find("[t]").expect("the header's theme switch");
    hover(&mut files, switch);
    assert_eq!(files.hovered().map(|hit| hit.command.clone()), Some(Command::CycleTheme));
    click(&mut files, switch);
    assert!(frame(&mut files).header().contains(&format!("[t] {}", THEMES[1].name)));

    let _ = fs::remove_dir_all(&config);
}
