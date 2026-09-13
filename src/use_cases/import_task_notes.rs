//! Restore a track task from `refs/notes/track` on the current branch.

use crate::db::Database;
use crate::models::{ScrapVisibility, Task, TodoAddOptions, TodoIndex, TodoStatus};
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, git_notes, git_worktree,
    task_notes::{ScrapNotesDto, TaskNotesDto, TodoNotesDto},
    todo_commit,
};
use crate::utils::{Result, TrackError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ImportTaskNotesOutcome {
    pub task: Task,
    pub slug: String,
    pub todos: usize,
    pub scraps: usize,
    pub links: usize,
    pub repo_registered: bool,
    pub alias_set: bool,
    pub ticket_skipped: Option<String>,
}

pub struct ImportTaskNotesUseCase<'a> {
    db: &'a Database,
}

impl<'a> ImportTaskNotesUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(&self, repo_path: &Path) -> Result<ImportTaskNotesOutcome> {
        let repo = repo_path
            .to_str()
            .ok_or_else(|| TrackError::PathResolutionFailed(repo_path.display().to_string()))?;
        if !git_worktree::is_git_repository(repo) && git_worktree::git_dir(repo).is_none() {
            return Err(TrackError::NotGitRepository(repo.to_string()));
        }

        let (marker, mut snapshot) =
            git_notes::find_snapshot_on_head(repo)?.ok_or(TrackError::NoTaskNotes)?;
        merge_todo_commits(repo, &marker, &mut snapshot)?;

        self.restore(snapshot, Some(repo_path))
    }

    pub(crate) fn restore(
        &self,
        snapshot: TaskNotesDto,
        repo_path: Option<&Path>,
    ) -> Result<ImportTaskNotesOutcome> {
        let task_service = TaskService::new(self.db);
        let mut ticket_skipped = None;
        let ticket = snapshot.ticket_id.as_deref().and_then(|raw| {
            match crate::models::TicketId::parse(raw) {
                Ok(_) => Some(raw),
                Err(_) => {
                    ticket_skipped = Some(format!("invalid ticket '{raw}'"));
                    None
                }
            }
        });

        let task = match task_service.create_task(
            &snapshot.name,
            snapshot.description.as_deref(),
            ticket,
            snapshot.ticket_url.as_deref(),
        ) {
            Ok(task) => task,
            Err(err @ TrackError::DuplicateTicket(_, _)) => return Err(err),
            Err(err) if ticket.is_some() => {
                ticket_skipped = Some(err.to_string());
                task_service.create_task(
                    &snapshot.name,
                    snapshot.description.as_deref(),
                    None,
                    None,
                )?
            }
            Err(err) => return Err(err),
        };

        let alias_set = task_service
            .set_alias(task.id, &snapshot.slug, false)
            .is_ok();

        let todo_service = TodoService::new(self.db);
        let mut todo_map: HashMap<i64, TodoIndex> = HashMap::new();
        for todo in &snapshot.todos {
            let options = TodoAddOptions {
                worktree_requested: false,
                requires_workspace: todo.requires_workspace,
            };
            let created = todo_service.add_todo(task.id, &todo.content, options)?;
            todo_map.insert(todo.index, created.task_index);
            if let Ok(status) = TodoStatus::from_str(&todo.status)
                && status != TodoStatus::Pending
            {
                todo_service.transition_status(created.id, status)?;
            }
        }

        let link_service = LinkService::new(self.db);
        let mut links = 0usize;
        for link in &snapshot.links {
            let title = if link.title.is_empty() {
                None
            } else {
                Some(link.title.as_str())
            };
            if link_service.add_link(task.id, &link.url, title).is_ok() {
                links += 1;
            }
        }

        let scrap_service = ScrapService::new(self.db);
        let mut scraps = 0usize;
        for scrap in &snapshot.scraps {
            let active = scrap
                .active_todo_id
                .and_then(|old| todo_map.get(&old).copied());
            scrap_service.import_scrap(
                task.id,
                &scrap.content,
                &scrap.created_at,
                active,
                ScrapVisibility::Shared,
            )?;
            scraps += 1;
        }

        let mut repo_registered = false;
        if let Some(path) = repo_path
            && let Some(path_str) = path.to_str()
        {
            let canonical = PathBuf::from(path_str)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(path_str));
            if let Some(canonical_str) = canonical.to_str() {
                repo_registered = RepoService::new(self.db)
                    .add_repo(task.id, canonical_str, None, None)
                    .is_ok();
            }
        }

        let task = task_service.get_task(task.id)?;
        Ok(ImportTaskNotesOutcome {
            todos: todo_map.len(),
            scraps,
            links,
            slug: snapshot.slug,
            task,
            repo_registered,
            alias_set,
            ticket_skipped,
        })
    }
}

fn merge_todo_commits(repo: &str, marker: &str, snapshot: &mut TaskNotesDto) -> Result<()> {
    let commits = todo_commit::list_todo_commits(repo, marker)?;
    if commits.is_empty() {
        return Ok(());
    }

    let mut todos = Vec::new();
    let mut scraps = Vec::new();
    for commit in commits {
        let (content, status) = match git_notes::read_todo_notes(repo, &commit.sha)? {
            Some(notes) => {
                for scrap in notes.scraps {
                    scraps.push(ScrapNotesDto {
                        active_todo_id: Some(commit.todo_index),
                        ..scrap
                    });
                }
                (notes.content, notes.status)
            }
            None => {
                let subject = commit_subject(repo, &commit.sha)
                    .unwrap_or_else(|| format!("TODO {}", commit.todo_index));
                (subject, "done".to_string())
            }
        };
        todos.push(TodoNotesDto {
            index: commit.todo_index,
            content,
            status,
            requires_workspace: true,
        });
    }
    snapshot.todos = todos;
    snapshot.scraps = scraps;
    Ok(())
}

fn commit_subject(repo: &str, sha: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", repo, "log", "-1", "--format=%s", sha])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let subject = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if subject.is_empty() {
        None
    } else {
        Some(subject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::task_notes::{
        FORMAT, LinkNotesDto, ScrapNotesDto, TaskNotesDto, TodoNotesDto, VERSION,
    };

    #[test]
    fn restore_imports_todos_and_shared_scraps() {
        let db = Database::new_in_memory().unwrap();
        let snapshot = TaskNotesDto {
            format: FORMAT.to_string(),
            version: VERSION,
            slug: "auth".to_string(),
            name: "Implement OAuth".to_string(),
            description: Some("tokens".to_string()),
            ticket_id: Some("NOTE-1".to_string()),
            ticket_url: None,
            todos: vec![
                TodoNotesDto {
                    index: 1,
                    content: "Design".to_string(),
                    status: "done".to_string(),
                    requires_workspace: false,
                },
                TodoNotesDto {
                    index: 2,
                    content: "Implement".to_string(),
                    status: "pending".to_string(),
                    requires_workspace: true,
                },
            ],
            links: vec![LinkNotesDto {
                index: 1,
                url: "https://example.com/rfc".to_string(),
                title: "RFC".to_string(),
            }],
            scraps: vec![ScrapNotesDto {
                index: 1,
                content: "chose bcrypt".to_string(),
                created_at: "2026-09-13T00:00:00+00:00".to_string(),
                active_todo_id: Some(2),
            }],
        };

        let outcome = ImportTaskNotesUseCase::new(&db)
            .restore(snapshot, None)
            .unwrap();
        assert_eq!(outcome.task.name, "Implement OAuth");
        assert_eq!(outcome.todos, 2);
        assert_eq!(outcome.scraps, 1);
        assert_eq!(outcome.links, 1);
        assert!(outcome.alias_set);

        let todos = TodoService::new(&db).list_todos(outcome.task.id).unwrap();
        assert_eq!(todos[0].status, TodoStatus::Done);
        assert_eq!(todos[1].status, TodoStatus::Pending);
        let scraps = ScrapService::new(&db).list_scraps(outcome.task.id).unwrap();
        assert!(scraps[0].visibility.is_shared());
        assert_eq!(scraps[0].content, "chose bcrypt");
    }
}
