use crate::db::Database;
use crate::models::{Todo, TodoStatus, VcsMode, jj_slug};
use crate::services::{
    RepoService, ScrapService, TaskRevisionService, TaskService, TodoService, WorktreeService,
    git_notes, task_notes, task_workspace, todo_commit,
};
use crate::utils::{Result, TrackError};

/// Result of completing a TODO, including optional workspace bookmark name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteTodoOutcome {
    pub task_index: crate::models::TodoIndex,
    pub merged_bookmark: Option<String>,
}

/// Completes a TODO: merges/removes JJ workspaces, then marks the TODO done in SQLite.
///
/// JJ operations cannot participate in the database transaction. The workflow therefore
/// completes external side effects first, then persists the terminal state. If the DB
/// update fails after a successful merge, a typed error is returned so the caller can
/// recover manually.
pub struct CompleteTodoUseCase<'a> {
    db: &'a Database,
}

impl<'a> CompleteTodoUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Complete the TODO identified by task-scoped `task_index` on `task_id`.
    pub fn execute(
        &self,
        task_id: crate::models::TaskId,
        task_index: crate::models::TodoIndex,
    ) -> Result<CompleteTodoOutcome> {
        let todo_service = TodoService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let todo = todo_service.get_todo_by_index(task_id, task_index)?;
        todo.status.complete()?;

        let merged_bookmark = worktree_service.complete_worktree_for_todo(todo.id)?;

        if self.db.get_aggressive_mode()?.is_on() {
            self.commit_todo_workspaces(task_id, &todo)?;
        }

        if let Err(err) = todo_service.mark_done(todo.id) {
            if let Some(bookmark) = merged_bookmark.clone() {
                return Err(TrackError::TodoCompletionDbFailed {
                    todo_index: task_index.as_i64(),
                    bookmark,
                    detail: err.to_string(),
                });
            }
            return Err(err);
        }

        Ok(CompleteTodoOutcome {
            task_index,
            merged_bookmark,
        })
    }

    fn commit_todo_workspaces(&self, task_id: crate::models::TaskId, todo: &Todo) -> Result<()> {
        let task = TaskService::new(self.db).get_task(task_id)?;
        let slug = jj_slug(&task);
        let branch = task_workspace::branch_name(&slug);
        let vcs_git = self.db.get_vcs_mode()? == VcsMode::Git;
        let scraps = ScrapService::new(self.db).list_scraps(task_id)?;
        let revisions = TaskRevisionService::new(self.db);
        let mut done = todo.clone();
        done.status = TodoStatus::Done;

        for repo in RepoService::new(self.db).list_repos(task_id)? {
            let Some(rev) = revisions.get(task_id, &repo.repo_path)? else {
                continue;
            };
            let workspace = task_workspace::workspace_path(&repo.repo_path, &slug);
            if !task_workspace::workspace_exists(&workspace) {
                continue;
            }
            let sha = todo_commit::commit_todo(
                vcs_git,
                &workspace,
                &branch,
                &rev.git_commit,
                todo.task_index.as_i64(),
                &todo.content,
                &slug,
            )?;
            let notes = task_notes::build_todo_notes(&slug, &done, &scraps);
            git_notes::write_todo_notes(&workspace, &sha, &notes)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TaskService;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn complete_todo_rejects_already_done() {
        let db = setup_db();
        let task_id = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap()
            .id;
        let todo_service = TodoService::new(&db);
        let todo = todo_service.add_todo(task_id, "Item", false).unwrap();
        todo_service.mark_done(todo.id).unwrap();

        let result = CompleteTodoUseCase::new(&db).execute(task_id, todo.task_index);
        assert!(matches!(
            result,
            Err(TrackError::InvalidStatusTransition { .. })
        ));
    }

    #[test]
    fn complete_todo_surfaces_db_failure_after_merge_with_typed_error() {
        let db = setup_db();
        let task_id = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap()
            .id;
        let todo_service = TodoService::new(&db);
        let todo = todo_service.add_todo(task_id, "Item", false).unwrap();
        todo_service.delete_todo(todo.id).unwrap();

        let result = CompleteTodoUseCase::new(&db).execute(task_id, todo.task_index);
        assert!(matches!(result, Err(TrackError::TodoIndexNotFound(_))));
    }
}
