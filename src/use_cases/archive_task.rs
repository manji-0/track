use crate::db::Database;
use crate::models::{Task, WorktreeId};
use crate::services::{TaskService, WorktreeService};
use crate::utils::{Result, TrackError};

/// A workspace with uncommitted JJ changes blocking archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyWorkspace {
    pub id: WorktreeId,
    pub path: String,
}

/// Blockers discovered before archive.
#[derive(Debug, Clone, Default)]
pub struct ArchiveBlockers {
    pub dirty_workspaces: Vec<DirtyWorkspace>,
}

/// Result of archiving a task and cleaning up workspaces.
#[derive(Debug, Clone)]
pub struct ArchiveTaskOutcome {
    pub task: Task,
    pub removed_workspaces: Vec<(WorktreeId, String)>,
    pub workspace_errors: Vec<String>,
}

/// CLI-facing lines after a successful archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveCompletionView {
    pub info_lines: Vec<String>,
    pub error_lines: Vec<String>,
    pub summary: String,
}

impl ArchiveTaskOutcome {
    pub fn completion_view(&self) -> ArchiveCompletionView {
        let mut info_lines = Vec::new();
        if !self.removed_workspaces.is_empty() {
            info_lines.push("Cleaning up workspaces...".to_string());
            for (id, path) in &self.removed_workspaces {
                info_lines.push(format!("  Removed workspace #{}: {}", id, path));
            }
        }

        let error_lines = self
            .workspace_errors
            .iter()
            .map(|err| format!("  Error removing workspace: {err}"))
            .collect();

        ArchiveCompletionView {
            info_lines,
            error_lines,
            summary: format!("Archived task #{}: {}", self.task.id, self.task.name),
        }
    }
}

/// Interactive archive flow step.
#[derive(Debug, Clone)]
pub enum ArchiveTaskStep {
    Completed(ArchiveTaskOutcome),
    NeedsConfirmation(ArchivePrompt),
}

/// Confirmation required before forcing archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePrompt {
    pub task_id: crate::models::TaskId,
    pub kind: ArchivePromptKind,
}

/// Reason archive needs explicit user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePromptKind {
    UncommittedWorkspaces(Vec<String>),
}

/// CLI-facing warning and prompt text for archive confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePromptView {
    pub warning_lines: Vec<String>,
    pub prompt: String,
    pub non_tty_hint: String,
}

impl ArchivePrompt {
    pub fn view(&self) -> ArchivePromptView {
        match &self.kind {
            ArchivePromptKind::UncommittedWorkspaces(workspaces) => {
                let mut warning_lines =
                    vec!["WARNING: The following workspaces have uncommitted changes:".to_string()];
                warning_lines.extend(workspaces.iter().map(|line| format!("  {line}")));
                warning_lines.push(String::new());
                ArchivePromptView {
                    warning_lines,
                    prompt: "Archive and remove workspaces anyway? [y/N]: ".to_string(),
                    non_tty_hint: "commit or discard workspace changes, or re-run with `track archive --force`".to_string(),
                }
            }
        }
    }
}

/// Archives a task after optionally removing JJ workspaces.
pub struct ArchiveTaskUseCase<'a> {
    db: &'a Database,
}

impl<'a> ArchiveTaskUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn resolve_task_id(&self, task_ref: Option<&str>) -> Result<crate::models::TaskId> {
        let task_service = TaskService::new(self.db);
        match task_ref {
            Some(r) => task_service.resolve_task_id(r),
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask),
        }
    }

    pub fn find_archive_blockers(&self, task_id: crate::models::TaskId) -> Result<ArchiveBlockers> {
        let worktree_service = WorktreeService::new(self.db);
        Ok(ArchiveBlockers {
            dirty_workspaces: self.find_dirty_track_workspaces(task_id, &worktree_service)?,
        })
    }

    fn find_dirty_track_workspaces(
        &self,
        task_id: crate::models::TaskId,
        worktree_service: &WorktreeService<'_>,
    ) -> Result<Vec<DirtyWorkspace>> {
        let worktrees = worktree_service.list_worktrees(task_id)?;

        let mut dirty = Vec::new();
        for worktree in worktrees {
            if !std::path::Path::new(&worktree.path).exists() {
                continue;
            }
            if worktree_service.has_uncommitted_changes(&worktree.path)? {
                dirty.push(DirtyWorkspace {
                    id: worktree.id,
                    path: worktree.path,
                });
            }
        }
        Ok(dirty)
    }

    /// Runs archive, returning either completion or a confirmation prompt.
    ///
    /// When `force` is true, blockers are ignored and archive proceeds immediately.
    pub fn run(&self, task_id: crate::models::TaskId, force: bool) -> Result<ArchiveTaskStep> {
        match self.execute(task_id, force) {
            Ok(outcome) => Ok(ArchiveTaskStep::Completed(outcome)),
            Err(TrackError::UncommittedWorkspaces(workspaces)) => {
                Ok(ArchiveTaskStep::NeedsConfirmation(ArchivePrompt {
                    task_id,
                    kind: ArchivePromptKind::UncommittedWorkspaces(workspaces),
                }))
            }
            Err(err) => Err(err),
        }
    }

    /// Archives after the user confirmed a [`ArchivePrompt`].
    pub fn confirm_and_run(&self, task_id: crate::models::TaskId) -> Result<ArchiveTaskOutcome> {
        self.execute(task_id, true)
    }

    /// Removes workspaces and archives the task.
    ///
    /// When `force` is false, returns [`TrackError::UncommittedWorkspaces`].
    /// Prefer [`Self::run`] for interactive flows.
    pub fn execute(
        &self,
        task_id: crate::models::TaskId,
        force: bool,
    ) -> Result<ArchiveTaskOutcome> {
        let task_service = TaskService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let task = task_service.get_task(task_id)?;
        let blockers = self.find_archive_blockers(task_id)?;

        if !force && !blockers.dirty_workspaces.is_empty() {
            return Err(TrackError::UncommittedWorkspaces(
                blockers
                    .dirty_workspaces
                    .iter()
                    .map(|ws| {
                        if ws.id.as_i64() > 0 {
                            format!("#{} {}", ws.id, ws.path)
                        } else {
                            ws.path.clone()
                        }
                    })
                    .collect(),
            ));
        }

        let worktrees = worktree_service.list_worktrees(task_id)?;
        let mut removed_workspaces = Vec::new();
        let mut workspace_errors = Vec::new();

        for worktree in worktrees {
            let result = worktree_service.remove_worktree(worktree.id, false);
            match result {
                Ok(()) => removed_workspaces.push((worktree.id, worktree.path)),
                Err(err) => workspace_errors.push(format!("#{}: {}", worktree.id, err)),
            }
        }

        if !force && !workspace_errors.is_empty() {
            return Err(TrackError::WorkspaceRemovalFailed(workspace_errors));
        }

        task_service.archive_task(task_id)?;

        Ok(ArchiveTaskOutcome {
            task,
            removed_workspaces,
            workspace_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskStatus;
    use std::fs;

    #[test]
    fn archive_task_marks_task_archived() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service
            .create_task("Archive me", None, None, None)
            .unwrap();

        let outcome = ArchiveTaskUseCase::new(&db).execute(task.id, true).unwrap();

        assert_eq!(outcome.task.id, task.id);
        let archived = task_service.get_task(task.id).unwrap();
        assert_eq!(archived.status, TaskStatus::Archived);
    }

    #[test]
    fn archive_removes_stale_worktree_record_when_path_missing() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        db.get_connection()
            .execute(
                "INSERT INTO worktrees (task_id, path, branch, base_repo, status, created_at, is_base)
                 VALUES (?1, ?2, ?3, ?4, 'active', ?5, 0)",
                rusqlite::params![
                    task.id,
                    "/tmp/missing-worktree",
                    "track/task-1",
                    "/tmp/missing-repo",
                    now,
                ],
            )
            .unwrap();

        let outcome = ArchiveTaskUseCase::new(&db)
            .execute(task.id, false)
            .unwrap();

        assert_eq!(outcome.removed_workspaces.len(), 1);
        let archived = task_service.get_task(task.id).unwrap();
        assert_eq!(archived.status, TaskStatus::Archived);
    }

    #[test]
    fn archive_aborts_when_workspace_removal_fails_without_force() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let worktree_path = temp_dir.path().join("stale-workspace");
        fs::create_dir(&worktree_path).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        db.get_connection()
            .execute(
                "INSERT INTO worktrees (task_id, path, branch, base_repo, status, created_at, is_base)
                 VALUES (?1, ?2, ?3, ?4, 'active', ?5, 0)",
                rusqlite::params![
                    task.id,
                    worktree_path.to_str().unwrap(),
                    "track/task-1",
                    "/tmp/missing-repo",
                    now,
                ],
            )
            .unwrap();

        let result = ArchiveTaskUseCase::new(&db).execute(task.id, false);
        assert!(
            result.is_ok(),
            "leftover non-vcs directories should be removed: {result:?}"
        );
        assert!(!worktree_path.exists());
        let archived = task_service.get_task(task.id).unwrap();
        assert_eq!(archived.status, TaskStatus::Archived);
    }

    #[test]
    fn archive_prompt_view_for_dirty_workspaces() {
        let prompt = ArchivePrompt {
            task_id: crate::models::TaskId::from_i64(1),
            kind: ArchivePromptKind::UncommittedWorkspaces(vec!["#1 /path".to_string()]),
        };
        let view = prompt.view();
        assert!(view.warning_lines[0].contains("uncommitted"));
        assert_eq!(view.prompt, "Archive and remove workspaces anyway? [y/N]: ");
    }

    #[test]
    fn completion_view_formats_removed_workspaces() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Done", None, None, None)
            .unwrap();
        let outcome = ArchiveTaskOutcome {
            task: task.clone(),
            removed_workspaces: vec![(WorktreeId::from_i64(7), "/tmp/wt".to_string())],
            workspace_errors: vec!["#8: failed".to_string()],
        };

        let view = outcome.completion_view();
        assert!(view.info_lines[0].contains("Cleaning up"));
        assert!(view.info_lines[1].contains("#7"));
        assert!(view.error_lines[0].contains("failed"));
        assert_eq!(view.summary, format!("Archived task #{}: Done", task.id));
    }
}
