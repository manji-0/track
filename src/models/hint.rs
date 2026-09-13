use crate::models::{AggressiveMode, NextAction, VcsMode, WorkspaceFacts};
use serde::Serialize;

/// Post-state and next-action hint shown after track commands (and in `--json`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandHint {
    pub vcs_mode: VcsMode,
    pub aggressive: bool,
    pub post_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
    pub next_reason: String,
}

impl CommandHint {
    pub fn from_workflow(
        vcs_mode: VcsMode,
        aggressive: AggressiveMode,
        facts: &WorkspaceFacts,
        next: &NextAction,
    ) -> Self {
        let post_state = match facts.workspace_path.as_deref() {
            Some(path) if facts.coding_workspace_ready => {
                format!("workspace ready at {path}")
            }
            Some(path) => format!("workspace missing at {path} — run track sync"),
            None if facts.total_repo_count == 0 => {
                "no repository registered (track repo add .)".to_string()
            }
            None => "no task workspace yet — run track sync".to_string(),
        };

        Self {
            vcs_mode,
            aggressive: aggressive.is_on(),
            post_state,
            next_command: next.command.clone(),
            next_reason: next.reason.clone(),
        }
    }

    /// Human footer written to stderr after a command.
    pub fn stderr_lines(&self) -> Vec<String> {
        let aggressive = if self.aggressive { "on" } else { "off" };
        let mut lines = vec![format!(
            "hint: vcs={} aggressive={} | {}",
            self.vcs_mode, aggressive, self.post_state
        )];
        if let Some(command) = &self.next_command {
            lines.push(format!("next: {command}"));
        }
        if !self.next_reason.is_empty() {
            lines.push(format!("      {}", self.next_reason));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NextActionKind, WorkspaceFacts};

    #[test]
    fn hint_describes_ready_workspace() {
        let facts = WorkspaceFacts {
            coding_workspace_ready: true,
            workspace_path: Some("/repo/.worktrees/fix".to_string()),
            total_repo_count: 1,
            ..WorkspaceFacts::default()
        };
        let next = crate::models::NextAction {
            kind: NextActionKind::RunCommand,
            command: Some("cd \"/repo/.worktrees/fix\"".to_string()),
            reason: "Work on TODO #1".to_string(),
        };
        let hint = CommandHint::from_workflow(VcsMode::Git, AggressiveMode::Off, &facts, &next);
        assert!(hint.post_state.contains("ready"));
        assert_eq!(
            hint.next_command.as_deref(),
            Some("cd \"/repo/.worktrees/fix\"")
        );
        let stderr = hint.stderr_lines();
        assert!(stderr[0].contains("vcs=git"));
        assert!(stderr[1].starts_with("next:"));
    }
}
