/// Options when creating a new TODO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoAddOptions {
    /// Legacy per-TODO worktree flag (deprecated; use jj-task per task instead).
    pub worktree_requested: bool,
    /// When true (default), a jj-task/git workspace is required before execute.
    pub requires_workspace: bool,
}

impl Default for TodoAddOptions {
    fn default() -> Self {
        Self {
            worktree_requested: false,
            requires_workspace: true,
        }
    }
}

impl TodoAddOptions {
    pub fn from_flags(worktree: bool, no_workspace: bool) -> Self {
        Self {
            worktree_requested: worktree,
            requires_workspace: !no_workspace,
        }
    }
}

impl From<bool> for TodoAddOptions {
    /// `true` selects legacy `--worktree` (deprecated).
    fn from(worktree_requested: bool) -> Self {
        Self::from_flags(worktree_requested, false)
    }
}
