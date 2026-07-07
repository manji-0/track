use crate::db::Database;
use crate::models::{jj_slug, Task, VcsMode};
use crate::services::{jj_task, RepoService, TaskService, WorktreeService};
use crate::utils::{Result, TrackError};

/// A workspace with uncommitted JJ changes blocking archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyWorkspace {
    pub id: i64,
    pub path: String,
}

/// Blockers discovered before archive.
#[derive(Debug, Clone, Default)]
pub struct ArchiveBlockers {
    pub dirty_workspaces: Vec<DirtyWorkspace>,
    pub jj_task_slug: Option<String>,
    pub jj_task_workspaces: Vec<String>,
}

/// Result of archiving a task and cleaning up workspaces.
#[derive(Debug, Clone)]
pub struct ArchiveTaskOutcome {
    pub task: Task,
    pub removed_workspaces: Vec<(i64, String)>,
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
    pub task_id: i64,
    pub kind: ArchivePromptKind,
}

/// Reason archive needs explicit user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePromptKind {
    UncommittedWorkspaces(Vec<String>),
    JjTaskNotCompleted {
        slug: String,
        workspaces: Vec<String>,
    },
}

/// CLI-facing warning and prompt text for archive confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePromptView {
    pub warning_lines: Vec<String>,
    pub prompt: String,
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
                }
            }
            ArchivePromptKind::JjTaskNotCompleted { slug, workspaces } => {
                let mut warning_lines = vec![
                    format!("WARNING: jj-task workspace '{slug}' is not marked done."),
                    format!("  Merge your PR with the $jj skill, then run: jj-task done {slug}"),
                ];
                warning_lines.extend(workspaces.iter().map(|path| format!("  {path}")));
                warning_lines.push(String::new());
                ArchivePromptView {
                    warning_lines,
                    prompt: "Archive the track task anyway (jj-task map unchanged)? [y/N]: "
                        .to_string(),
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

    pub fn resolve_task_id(&self, task_ref: Option<&str>) -> Result<i64> {
        let task_service = TaskService::new(self.db);
        match task_ref {
            Some(r) => task_service.resolve_task_id(r),
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask),
        }
    }

    pub fn find_archive_blockers(&self, task_id: i64) -> Result<ArchiveBlockers> {
        let worktree_service = WorktreeService::new(self.db);
        let mut blockers = ArchiveBlockers {
            dirty_workspaces: self.find_dirty_track_workspaces(task_id, &worktree_service)?,
            ..Default::default()
        };

        if self.db.get_vcs_mode()? != VcsMode::Jj {
            return Ok(blockers);
        }

        let task_service = TaskService::new(self.db);
        let task = task_service.get_task(task_id)?;
        let repo_service = RepoService::new(self.db);
        let repos = repo_service.list_repos(task_id)?;
        if repos.is_empty() {
            return Ok(blockers);
        }

        let slug = jj_slug(&task);
        let repo_paths: Vec<String> = repos.iter().map(|repo| repo.repo_path.clone()).collect();
        let active = jj_task::active_registrations(&slug, &repo_paths);
        if !active.is_empty() {
            blockers.jj_task_slug = Some(slug.clone());
            blockers.jj_task_workspaces = active
                .iter()
                .filter_map(|status| status.workspace_path.clone())
                .collect();
        }

        for path in jj_task::active_workspace_paths(&slug, &repo_paths) {
            if std::path::Path::new(&path).exists()
                && worktree_service.has_uncommitted_changes(&path)?
                && !blockers.dirty_workspaces.iter().any(|ws| ws.path == path)
            {
                blockers
                    .dirty_workspaces
                    .push(DirtyWorkspace { id: 0, path });
            }
        }

        Ok(blockers)
    }

    fn find_dirty_track_workspaces(
        &self,
        task_id: i64,
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
    pub fn run(&self, task_id: i64, force: bool) -> Result<ArchiveTaskStep> {
        match self.execute(task_id, force) {
            Ok(outcome) => Ok(ArchiveTaskStep::Completed(outcome)),
            Err(TrackError::UncommittedWorkspaces(workspaces)) => {
                Ok(ArchiveTaskStep::NeedsConfirmation(ArchivePrompt {
                    task_id,
                    kind: ArchivePromptKind::UncommittedWorkspaces(workspaces),
                }))
            }
            Err(TrackError::JjTaskNotCompleted { slug, workspaces }) => {
                Ok(ArchiveTaskStep::NeedsConfirmation(ArchivePrompt {
                    task_id,
                    kind: ArchivePromptKind::JjTaskNotCompleted { slug, workspaces },
                }))
            }
            Err(err) => Err(err),
        }
    }

    /// Archives after the user confirmed a [`ArchivePrompt`].
    pub fn confirm_and_run(&self, task_id: i64) -> Result<ArchiveTaskOutcome> {
        self.execute(task_id, true)
    }

    /// Removes workspaces and archives the task.
    ///
    /// When `force` is false, returns [`TrackError::UncommittedWorkspaces`] or
    /// [`TrackError::JjTaskNotCompleted`]. Prefer [`Self::run`] for interactive flows.
    pub fn execute(&self, task_id: i64, force: bool) -> Result<ArchiveTaskOutcome> {
        let task_service = TaskService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let task = task_service.get_task(task_id)?;
        let blockers = self.find_archive_blockers(task_id)?;

        if !force {
            if let Some(slug) = &blockers.jj_task_slug {
                return Err(TrackError::JjTaskNotCompleted {
                    slug: slug.clone(),
                    workspaces: blockers.jj_task_workspaces.clone(),
                });
            }

            if !blockers.dirty_workspaces.is_empty() {
                return Err(TrackError::UncommittedWorkspaces(
                    blockers
                        .dirty_workspaces
                        .iter()
                        .map(|ws| {
                            if ws.id > 0 {
                                format!("#{} {}", ws.id, ws.path)
                            } else {
                                format!("jj-task {}", ws.path)
                            }
                        })
                        .collect(),
                ));
            }
        }

        let worktrees = worktree_service.list_worktrees(task_id)?;
        let mut removed_workspaces = Vec::new();
        let mut workspace_errors = Vec::new();

        for worktree in worktrees {
            match worktree_service.remove_worktree(worktree.id, force) {
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
    use crate::services::jj_task;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn jj_task_map_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn insert_repo(db: &Database, task_id: i64, repo_path: &str) {
        db.get_connection()
            .execute(
                "INSERT INTO task_repos (task_id, task_index, repo_path, created_at) VALUES (?1, 1, ?2, datetime('now'))",
                rusqlite::params![task_id, repo_path],
            )
            .unwrap();
    }

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
    fn find_archive_blockers_detects_active_jj_task() {
        let _guard = jj_task_map_lock();
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        let repo_path = temp.path().join("repo");
        fs::create_dir_all(&repo_path).unwrap();
        let repo_key = jj_task::repo_key(repo_path.to_str().unwrap());
        fs::write(
            &map_path,
            format!(
                r#"{{
              "repos": {{
                {repo_key:?}: {{
                  "tasks": {{
                    "task-1": {{
                      "workspace": "/repo/.worktrees/task-1",
                      "phase": "active"
                    }}
                  }}
                }}
              }}
            }}"#
            ),
        )
        .unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();
        insert_repo(&db, task.id, repo_path.to_str().unwrap());

        let blockers = ArchiveTaskUseCase::new(&db)
            .find_archive_blockers(task.id)
            .unwrap();

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }

        assert_eq!(blockers.jj_task_slug.as_deref(), Some("task-1"));
        assert!(!blockers.jj_task_workspaces.is_empty());
    }

    #[test]
    fn archive_allows_done_jj_task_phase() {
        let _guard = jj_task_map_lock();
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        let repo_path = temp.path().join("repo");
        fs::create_dir_all(&repo_path).unwrap();
        let repo_key = jj_task::repo_key(repo_path.to_str().unwrap());
        fs::write(
            &map_path,
            format!(
                r#"{{
              "repos": {{
                {repo_key:?}: {{
                  "tasks": {{
                    "task-1": {{
                      "workspace": "/repo/.worktrees/task-1",
                      "phase": "done"
                    }}
                  }}
                }}
              }}
            }}"#
            ),
        )
        .unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();
        insert_repo(&db, task.id, repo_path.to_str().unwrap());

        let result = ArchiveTaskUseCase::new(&db).execute(task.id, false);

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }

        assert!(result.is_ok());
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
            matches!(
                result,
                Err(TrackError::WorkspaceRemovalFailed(_)) | Err(TrackError::Jj(_))
            ),
            "unexpected result: {result:?}"
        );

        let still_active = task_service.get_task(task.id).unwrap();
        assert_eq!(still_active.status, TaskStatus::Active);
    }

    #[test]
    fn archive_prompt_view_for_dirty_workspaces() {
        let prompt = ArchivePrompt {
            task_id: 1,
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
            removed_workspaces: vec![(7, "/tmp/wt".to_string())],
            workspace_errors: vec!["#8: failed".to_string()],
        };

        let view = outcome.completion_view();
        assert!(view.info_lines[0].contains("Cleaning up"));
        assert!(view.info_lines[1].contains("#7"));
        assert!(view.error_lines[0].contains("failed"));
        assert_eq!(view.summary, format!("Archived task #{}: Done", task.id));
    }

    #[test]
    fn run_returns_prompt_for_active_jj_task() {
        let _guard = jj_task_map_lock();
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        let repo_path = temp.path().join("repo");
        fs::create_dir_all(&repo_path).unwrap();
        let repo_key = jj_task::repo_key(repo_path.to_str().unwrap());
        fs::write(
            &map_path,
            format!(
                r#"{{
              "repos": {{
                {repo_key:?}: {{
                  "tasks": {{
                    "task-1": {{
                      "workspace": "/repo/.worktrees/task-1",
                      "phase": "active"
                    }}
                  }}
                }}
              }}
            }}"#
            ),
        )
        .unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();
        insert_repo(&db, task.id, repo_path.to_str().unwrap());

        let step = ArchiveTaskUseCase::new(&db).run(task.id, false).unwrap();

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }

        match step {
            ArchiveTaskStep::NeedsConfirmation(prompt) => {
                let view = prompt.view();
                assert!(view.warning_lines[0].contains("jj-task"));
                assert!(view.prompt.contains("jj-task map unchanged"));
            }
            ArchiveTaskStep::Completed(_) => panic!("expected confirmation prompt"),
        }
    }
}
