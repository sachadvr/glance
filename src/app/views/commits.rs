use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, Paragraph};
use ratatui::Frame;

use super::{CommitFilesView, ListCursor};
use crate::app::command::Command;
use crate::app::keymap::{self, char, Binding};
use crate::app::session::Session;
use crate::app::view::{Context, Paint, TabTitle, Tool, Transition, View};
use crate::domain::Commit;
use crate::i18n::Strings;
use crate::theme::Theme;
use crate::ui::{draw_list, every_row, fit_end, grey, stat_line, stat_spans, Hit, StatColumns};

const MAX_COMMIT_FILES_SHOWN: usize = 12;
const COMMIT_FILES_INDENT: &str = "   ";

/// The branch's commits (or what a log, a reflog, a stash list piped), each optionally unfolded into its files.
#[derive(Default)]
pub struct CommitsView {
    cursor: ListCursor,
    /// Whether each commit unfolds its files under it, as `git log --stat` does.
    shows_files: bool,
}

impl View for CommitsView {
    fn keymap(&self, strings: &'static Strings) -> Vec<Binding> {
        let mut bindings = keymap::list_motions();
        bindings.push(keymap::open(strings, strings.open_commit_files));
        bindings.push(Binding::new(Command::ToggleCommitFiles, &[char('s')]).hint("s", strings.unfold));
        bindings
    }

    fn handle(&mut self, command: Command, context: &mut Context) -> Transition {
        let length = context.session.commits.len();
        match command {
            Command::Move(motion) => self.cursor.apply(motion, length),
            Command::ToggleCommitFiles => self.shows_files = !self.shows_files,
            Command::Open if length > 0 => return open_commit(context, self.cursor.within(length)),
            _ => {}
        }
        Transition::Stay
    }

    // Unfolded, the list needs the files of the commits around the cursor before it is drawn.
    fn prepare(&mut self, context: &mut Context) {
        if !self.shows_files {
            return;
        }
        let first = self.cursor.index.saturating_sub(self.cursor.page);
        let last = (self.cursor.index + self.cursor.page).min(context.session.commits.len());
        for index in first..last {
            if let Err(error) = context.session.commit_files(index) {
                context.fail_git(&error);
                self.shows_files = false;
                return;
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, paint: &Paint) -> Vec<Hit> {
        self.cursor.drawn(area);
        let commits = &paint.session.commits;
        if commits.is_empty() {
            frame.render_widget(Paragraph::new(grey(format!(" {}", paint.strings.no_commits), paint.theme)), area);
            return Vec::new();
        }
        let width = usize::from(area.width).saturating_sub(1);
        let items = commits.iter().map(|commit| commit_item(commit, width, self.shows_files, paint)).collect();
        draw_list(frame, area, items, self.cursor.within(commits.len()), &every_row(commits.len()), paint.theme)
    }

    fn tab_title(&self, session: &Session, strings: &'static Strings) -> Option<TabTitle> {
        let label = if session.commits.first().is_some_and(Commit::is_stash) { strings.stash_tab } else { strings.commits_tab };
        Some(TabTitle { key: 'c', label, count: session.commits.len(), stats: session.commits.iter().map(Commit::stats).sum() })
    }

    fn tools(&self, theme: &Theme, strings: &Strings) -> Vec<Tool> {
        let (marker, color) = if self.shows_files { ("▾", theme.accent) } else { ("▸", theme.grey) };
        vec![Tool { span: Span::styled(format!("[s] {} {marker}", strings.commit_files_tool), Style::new().fg(color)), command: Command::ToggleCommitFiles }]
    }

    fn search_texts(&self, session: &Session) -> Vec<String> {
        session.commits.iter().map(commit_search_text).collect()
    }

    fn position(&self) -> usize {
        self.cursor.index
    }

    fn select(&mut self, index: usize) {
        self.cursor.index = index;
    }

    fn yank(&self, session: &Session, _whole: bool) -> Option<String> {
        session.commits.get(self.cursor.within(session.commits.len())).map(|commit| commit.revision.as_str().to_string())
    }
}

fn open_commit(context: &mut Context, index: usize) -> Transition {
    match context.session.commit_files(index) {
        Ok(_) => Transition::Push(Box::new(CommitFilesView::new(context.session.commits[index].clone()))),
        Err(error) => {
            context.fail_git(&error);
            Transition::Stay
        }
    }
}

// What a search matches in a commit: the hash, subject and author, one per line so that a query never spans two.
pub(super) fn commit_search_text(commit: &Commit) -> String {
    format!("{}\n{}\n{}", commit.revision.as_str(), commit.subject, commit.author)
}

fn commit_item(commit: &Commit, width: usize, shows_files: bool, paint: &Paint) -> ListItem<'static> {
    let theme = paint.theme;
    let mut facts: Vec<String> = commit.byline().into_iter().map(String::from).collect();
    // Files not loaded yet are unknown, not zero.
    if let Some(files) = &commit.files {
        facts.push((paint.strings.file_count)(files.len()));
    }
    let mut details = vec![grey(format!("   {}", facts.join(" · ")), theme)];
    details.extend(stat_spans(commit.stats(), theme));

    let hash = commit.revision.as_str();
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!(" {hash} "), Style::new().fg(theme.accent)),
            Span::raw(fit_end(&commit.subject, width.saturating_sub(hash.len() + 2))).bold(),
        ]),
        Line::from(details),
    ];
    if shows_files {
        lines.extend(commit_file_lines(commit, width, paint));
    }
    lines.push(Line::default());
    ListItem::new(lines)
}

// Under an unfolded commit: its files as stat rows, the longest commits cut short.
fn commit_file_lines(commit: &Commit, width: usize, paint: &Paint) -> Vec<Line<'static>> {
    let theme = paint.theme;
    let files = commit.files.as_deref().unwrap_or_default();
    let columns = StatColumns::fit(files, width, COMMIT_FILES_INDENT.len());
    let mut lines: Vec<Line> = files
        .iter()
        .take(MAX_COMMIT_FILES_SHOWN)
        .map(|file| {
            let mut line = stat_line(file, columns, theme);
            line.spans.insert(0, Span::raw(COMMIT_FILES_INDENT));
            line
        })
        .collect();
    let hidden = files.len().saturating_sub(MAX_COMMIT_FILES_SHOWN);
    if hidden > 0 {
        lines.push(Line::from(grey(format!("{COMMIT_FILES_INDENT}  {}", (paint.strings.more_files)(hidden)), theme)));
    }
    lines
}
