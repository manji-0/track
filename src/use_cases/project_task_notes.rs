//! Project task metadata onto the marker and per-TODO notes onto unpublished commits.

use crate::db::Database;
use crate::models::{TaskId, jj_slug};
use crate::services::{
    LinkService, ScrapService, TaskRevisionService, TaskService, TodoService, git_notes,
    task_notes, task_workspace, todo_commit,
};
use crate::utils::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectTaskNotesOutcome {
    Skipped,
    Written { markers: usize },
}

pub struct ProjectTaskNotesUseCase<'a> {
    db: &'a Database,
}

impl<'a> ProjectTaskNotesUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(&self, task_id: TaskId) -> Result<ProjectTaskNotesOutcome> {
        if !self.db.get_aggressive_mode()?.is_on() {
            return Ok(ProjectTaskNotesOutcome::Skipped);
        }

        let task = TaskService::new(self.db).get_task(task_id)?;
        let slug = jj_slug(&task);
        let branch = task_workspace::branch_name(&slug);
        let links = LinkService::new(self.db).list_links(task_id)?;
        let scraps = ScrapService::new(self.db).list_scraps(task_id)?;
        let todos = TodoService::new(self.db).list_todos(task_id)?;
        let marker_snapshot = task_notes::build_marker_snapshot(&slug, &task, &links);
        let revisions = TaskRevisionService::new(self.db).list_for_task(task_id)?;
        if revisions.is_empty() {
            return Ok(ProjectTaskNotesOutcome::Skipped);
        }

        let mut markers = 0usize;
        for rev in revisions {
            let workspace = task_workspace::workspace_path(&rev.repo_path, &slug);
            let root = if task_workspace::workspace_exists(&workspace) {
                workspace.as_str()
            } else {
                rev.repo_path.as_str()
            };

            let marker_published = todo_commit::is_published(root, &rev.git_commit, &branch);
            let marker_has_note =
                git_notes::read_notes(root, &rev.git_commit)?.is_some_and(|body| !body.is_empty());
            if !marker_published || !marker_has_note {
                git_notes::write_snapshot(root, &rev.git_commit, &marker_snapshot)?;
            }

            for commit in todo_commit::list_todo_commits(root, &rev.git_commit)? {
                if todo_commit::is_published(root, &commit.sha, &branch) {
                    continue;
                }
                let Some(todo) = todos
                    .iter()
                    .find(|t| t.task_index.as_i64() == commit.todo_index)
                else {
                    continue;
                };
                let notes = task_notes::build_todo_notes(&slug, todo, &scraps);
                git_notes::write_todo_notes(root, &commit.sha, &notes)?;
            }
            markers += 1;
        }
        Ok(ProjectTaskNotesOutcome::Written { markers })
    }
}

/// Best-effort projection used by CLI/WebUI mutations.
pub fn project_task_notes_or_warn(db: &Database, task_id: TaskId) {
    if let Err(err) = ProjectTaskNotesUseCase::new(db).execute(task_id) {
        eprintln!("warning: git notes not updated: {err}");
    }
}
