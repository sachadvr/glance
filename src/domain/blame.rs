/// A line of `git blame`: who last wrote it, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameLine {
    pub hash: String,
    pub author: String,
    /// "2026-10-16": the day is what a glance needs.
    pub date: String,
    pub number: usize,
    pub code: String,
}

impl BlameLine {
    /// git shows a line not committed yet with an all-zero hash.
    pub fn is_committed(&self) -> bool {
        self.hash.bytes().any(|byte| byte != b'0')
    }
}

/// A blamed file: its lines, and its path when it could be found (it picks the grammar the code is coloured with).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blame {
    pub path: Option<String>,
    pub lines: Vec<BlameLine>,
}
