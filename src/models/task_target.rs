use serde::Serialize;
use std::path::PathBuf;

/// Where the task a command operates on came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetSource {
    /// The global `--task` flag (or a command's positional reference).
    Flag,
    /// The `TRACK_TASK` environment variable.
    Env,
    /// The working directory is inside the task's `.worktrees/<slug>` workspace.
    Workspace,
    /// The global current task (single context mode only).
    Current,
}

impl TargetSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Env => "env",
            Self::Workspace => "workspace",
            Self::Current => "current",
        }
    }
}

/// Per-session inputs read once at the CLI boundary (`TRACK_TASK`, working directory).
#[derive(Debug, Clone, Default)]
pub struct SessionBinding {
    pub env_task: Option<String>,
    pub cwd: Option<PathBuf>,
}

impl SessionBinding {
    /// `TRACK_TASK` when set to a non-empty value.
    pub fn env_task_ref(&self) -> Option<&str> {
        self.env_task.as_deref().filter(|r| !r.trim().is_empty())
    }
}
