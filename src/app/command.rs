use crate::git::Base;

/// How a cursor or a scroll moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Up,
    Down,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Top,
    Bottom,
}

impl Motion {
    /// Where `position` lands among `length` items, `page` of them fitting on screen.
    pub fn apply(self, position: usize, length: usize, page: usize) -> usize {
        let last = length.saturating_sub(1);
        match self {
            Motion::Up => position.saturating_sub(1),
            Motion::Down => (position + 1).min(last),
            Motion::PageUp => position.saturating_sub(page),
            Motion::PageDown => (position + page).min(last),
            Motion::HalfPageUp => position.saturating_sub(page / 2),
            Motion::HalfPageDown => (position + page / 2).min(last),
            Motion::Top => 0,
            Motion::Bottom => last,
        }
    }
}

/// Everything a key or a click can ask for. Keys reach a command through a keymap; clicks carry one directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Move(Motion),
    Open,
    /// A click on a row: select it, then open it.
    OpenAt(usize),
    Back,
    NextFile,
    PreviousFile,
    ToggleWrap,
    ToggleCommitFiles,
    SetBase(Base),
    PickBase,
    Reload,
    Fetch,
    StartSearch,
    SearchNext,
    SearchPrevious,
    ClearSearch,
    Yank,
    YankAll,
    CycleTheme,
    SwitchTab(usize),
    NextTab,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motions_stay_inside_the_list() {
        assert_eq!(Motion::Down.apply(4, 5, 10), 4);
        assert_eq!(Motion::Up.apply(0, 5, 10), 0);
        assert_eq!(Motion::PageDown.apply(1, 50, 10), 11);
        assert_eq!(Motion::HalfPageUp.apply(3, 50, 10), 0);
        assert_eq!(Motion::Bottom.apply(0, 0, 10), 0);
    }
}
