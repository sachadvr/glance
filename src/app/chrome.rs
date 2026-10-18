//! What frames every screen: the header, the tab row, the base row and the footer.

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::command::Command;
use super::keymap;
use super::session::{FetchState, Source};
use super::view::Paint;
use super::{App, Status};
use crate::git::Base;
use crate::theme::Theme;
use crate::ui::{grey, stat_spans, Hit, Row};

pub(super) fn draw(frame: &mut Frame, app: &mut App) -> Vec<Hit> {
    let theme = app.theme.current();
    let [header, tabs, body, footer] = Layout::vertical([Constraint::Length(1), Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    if let Some((background, foreground)) = theme.canvas {
        let area = frame.area();
        frame.buffer_mut().set_style(area, Style::new().bg(background).fg(foreground));
    }

    let header_row = header_row(app, usize::from(header.width), theme);
    let tab_row = tab_row(app, theme);
    let base_row = base_row(app, theme);
    let mut hits: Vec<Hit> = header_row.hits(header, 0).chain(tab_row.hits(tabs, 0)).chain(base_row.hits(tabs, 1)).collect();
    frame.render_widget(Paragraph::new(Line::from(header_row.spans)), header);
    frame.render_widget(Paragraph::new(vec![Line::from(tab_row.spans), Line::from(base_row.spans)]), tabs);
    frame.render_widget(Paragraph::new(footer_line(app, theme)), footer);

    let paint = Paint { session: &app.session, settings: app.settings, theme, strings: app.strings, search: app.search.query.as_deref() };
    let top = app.tabs[app.active].last_mut().expect("a tab keeps its first screen");
    hits.extend(top.render(frame, body, &paint));
    paint_hover(frame, app, &hits, theme);
    hits
}

fn paint_hover(frame: &mut Frame, app: &App, hits: &[Hit], theme: &Theme) {
    let Some(hovered) = app.hover.and_then(|hover| hits.iter().find(|hit| hit.area.contains(hover))) else {
        return;
    };
    let style = match hovered.command {
        Command::OpenAt(_) => Style::new().bg(theme.hover_bg),
        // A button may already sit on a coloured background: hovering underlines it rather than repaint it.
        _ => Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    };
    frame.buffer_mut().set_style(hovered.area, style);
}

fn header_row(app: &App, width: usize, theme: &Theme) -> Row {
    let mut row = Row::default();
    row.push(Span::styled(" glance ", Style::new().bg(theme.accent).fg(theme.on_accent).bold()));
    match &app.session.source {
        Source::Pager { command } => row.push(grey(format!("  {command}"), theme)),
        Source::Repo(repo) => {
            row.push(Span::raw(format!("  {}  ", repo.name)).bold());
            row.push(Span::styled(format!("⎇ {}", repo.branch), Style::new().fg(theme.blue)));
            row.push(match app.session.fetch {
                FetchState::Idle => Span::raw(""),
                FetchState::Running => grey(format!("   {}", app.strings.fetch_running), theme),
                FetchState::Done => Span::styled(format!("   {}", app.strings.fetch_done), Style::new().fg(theme.green)),
                FetchState::Failed => Span::styled(format!("   {}", app.strings.fetch_failed), Style::new().fg(theme.red)),
            });
        }
    }
    let switch = grey(format!("[t] {} ", theme.name), theme);
    row.push(Span::raw(" ".repeat(width.saturating_sub(row.width() + switch.content.chars().count()).max(3))));
    row.push_target(switch, Command::CycleTheme);
    row
}

fn tab_row(app: &App, theme: &Theme) -> Row {
    let mut row = Row::default();
    row.push(Span::raw(" "));
    for (index, stack) in app.tabs.iter().enumerate() {
        let Some(title) = stack[0].tab_title(&app.session, app.strings) else {
            continue;
        };
        let style = if index == app.active { Style::new().fg(theme.accent).bold().underlined() } else { Style::new().fg(theme.grey) };
        let span = Span::styled(format!("[{}] {} {}", title.key, title.label, title.count), style);
        // A tab switches only from its root: inside a commit or a diff, the click would land nowhere.
        if app.is_at_root() {
            row.push_target(span, Command::SwitchTab(index));
        } else {
            row.push(span);
        }
        stat_spans(title.stats, theme).into_iter().for_each(|span| row.push(span));
        row.push(Span::raw("    "));
    }
    if app.is_at_root() {
        for tool in app.top().tools(theme, app.strings) {
            row.push_target(tool.span, tool.command);
        }
    }
    row
}

fn base_row(app: &App, theme: &Theme) -> Row {
    let mut row = Row::default();
    let Some(repo) = app.session.repo() else {
        return row;
    };
    if !app.top().shows_base() {
        return row;
    }
    let base = &app.session.base;
    let is_picking = !app.is_at_root();
    let other = match base {
        Base::Commit(hash) => (app.strings.since_commit)(hash),
        _ => app.strings.other_base.to_string(),
    };
    // While a commit is being picked, or without a main line, an option shows but does not click.
    let options = [
        ('1', app.strings.worktree.to_string(), *base == Base::Worktree && !is_picking, (!is_picking).then_some(Command::SetBase(Base::Worktree))),
        ('2', repo.mainline.clone().unwrap_or_else(|| "—".to_string()), *base == Base::Mainline && !is_picking, (!is_picking && repo.mainline.is_some()).then_some(Command::SetBase(Base::Mainline))),
        ('3', other, matches!(base, Base::Commit(_)) || is_picking, (!is_picking).then_some(Command::PickBase)),
    ];
    row.push(grey(format!(" {}  ", app.strings.since), theme));
    for (key, label, is_active, command) in options {
        let style = if is_active { Style::new().bg(theme.accent).fg(theme.on_accent).bold() } else { Style::new().fg(theme.grey) };
        let span = Span::styled(format!(" {key} {label} "), style);
        match command {
            Some(command) => row.push_target(span, command),
            None => row.push(span),
        }
        row.push(Span::raw(" "));
    }
    row
}

fn footer_line(app: &App, theme: &Theme) -> Line<'static> {
    if let Some(input) = &app.search.input {
        return Line::from(vec![Span::styled(format!(" /{input}"), Style::new().fg(theme.accent).bold()), Span::styled(" ", Style::new().bg(theme.accent))]);
    }
    match &app.status {
        Some(Status::Error(message)) => return Line::from(Span::styled(format!(" {message}"), Style::new().fg(theme.red))),
        Some(Status::Notice(notice)) => return Line::from(Span::styled(format!(" {notice}"), Style::new().fg(theme.green))),
        None => {}
    }
    if let Some(query) = &app.search.query {
        return Line::from(Span::styled(format!(" {}", (app.strings.search_status)(query)), Style::new().fg(theme.accent)));
    }
    let layers = app.layers();
    Line::from(grey(format!(" {}", keymap::hints(layers.iter().flatten())), theme))
}
