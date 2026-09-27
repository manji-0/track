use crate::models::TaskId;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// How commands pick the task they operate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContextMode {
    /// One implicit current task (`track switch`). Default.
    #[default]
    Single,
    /// Several tasks in parallel. Every task-scoped command needs `--task <ref>`.
    Multi,
}

impl ContextMode {
    pub const KEY: &'static str = "context_mode";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Multi => "multi",
        }
    }

    pub fn is_multi(self) -> bool {
        matches!(self, Self::Multi)
    }

    /// Rewrites a suggested `track ...` command so it targets `task_id` explicitly in multi mode.
    pub fn scope_command(self, task_id: TaskId, command: &str) -> String {
        match (self, command.strip_prefix("track ")) {
            (Self::Multi, Some(rest)) => format!("track --task {task_id} {rest}"),
            _ => command.to_string(),
        }
    }
}

impl fmt::Display for ContextMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ContextMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "single" => Ok(Self::Single),
            "multi" => Ok(Self::Multi),
            other => Err(format!(
                "unknown context mode '{other}' (expected 'single' or 'multi')"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_context_mode() {
        assert_eq!(
            "single".parse::<ContextMode>().unwrap(),
            ContextMode::Single
        );
        assert_eq!(
            " Multi ".parse::<ContextMode>().unwrap(),
            ContextMode::Multi
        );
        assert!("both".parse::<ContextMode>().is_err());
    }

    #[test]
    fn scope_command_prefixes_task_only_in_multi_mode() {
        let id = TaskId::from_i64(7);
        assert_eq!(
            ContextMode::Multi.scope_command(id, "track todo done 1"),
            "track --task 7 todo done 1"
        );
        assert_eq!(
            ContextMode::Single.scope_command(id, "track todo done 1"),
            "track todo done 1"
        );
        assert_eq!(
            ContextMode::Multi.scope_command(id, "cd \"/repo/.worktrees/x\""),
            "cd \"/repo/.worktrees/x\""
        );
    }
}
