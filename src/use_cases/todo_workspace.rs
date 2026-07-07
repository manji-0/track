use crate::db::Database;
use crate::models::TaskRepo;
use crate::services::{RepoService, TaskService, TodoService, WorktreeService};
use crate::utils::{Result, TrackError};
use std::path::{Path, PathBuf};

/// Options for creating or showing a TODO workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoWorkspaceRequest {
    pub recreate: bool,
    pub force: bool,
    pub all_repos: bool,
}

/// Result of resolving or creating TODO workspaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoWorkspaceOutcome {
    pub paths: Vec<String>,
    pub warnings: Vec<String>,
}

/// Creates, recreates, or lists jj workspaces for a TODO.
pub struct TodoWorkspaceUseCase<'a> {
    db: &'a Database,
}

impl<'a> TodoWorkspaceUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(
        &self,
        task_id: i64,
        todo_index: i64,
        request: TodoWorkspaceRequest,
    ) -> Result<TodoWorkspaceOutcome> {
        let todo_service = TodoService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);
        let repo_service = RepoService::new(self.db);
        let task_service = TaskService::new(self.db);

        let todo = todo_service.get_todo_by_index(task_id, todo_index)?;
        let repos = repo_service.list_repos(task_id)?;

        if repos.is_empty() {
            return Err(TrackError::NoRepositoriesRegistered);
        }

        let target_repos = if request.all_repos {
            repos
        } else {
            vec![resolve_current_repo(&repos)?]
        };

        let worktrees = worktree_service.list_worktrees(task_id)?;
        let task = task_service.get_task(task_id)?;
        let branch_name = worktree_service.get_todo_branch_name(
            task_id,
            task.ticket_id.as_deref(),
            todo.task_index,
        )?;

        let mut paths = Vec::new();
        let mut warnings = Vec::new();

        for repo in target_repos {
            let mut todo_worktrees: Vec<_> = worktrees
                .iter()
                .filter(|wt| {
                    wt.todo_id == Some(todo.id)
                        && wt.base_repo.as_deref() == Some(repo.repo_path.as_str())
                })
                .cloned()
                .collect();

            if !todo_worktrees.is_empty() {
                if !request.recreate {
                    paths.extend(todo_worktrees.iter().map(|worktree| worktree.path.clone()));
                    continue;
                }

                if !request.force {
                    for worktree in &todo_worktrees {
                        if Path::new(&worktree.path).exists()
                            && worktree_service.has_uncommitted_changes(&worktree.path)?
                        {
                            return Err(TrackError::WorkspaceHasUncommittedChanges {
                                path: worktree.path.clone(),
                            });
                        }
                    }
                }

                for worktree in todo_worktrees.drain(..) {
                    if !worktree_service
                        .bookmark_exists_in_repo(repo.repo_path.as_str(), &worktree.branch)?
                    {
                        if Path::new(&worktree.path).exists() {
                            paths.push(worktree.path.clone());
                        }
                        if request.all_repos {
                            warnings.push(format!(
                                "Skipping recreate for {} (missing branch/bookmark).",
                                worktree.path
                            ));
                            continue;
                        }
                        return Err(TrackError::BookmarkNotFound {
                            bookmark: worktree.branch.clone(),
                            repo_path: repo.repo_path.clone(),
                        });
                    }

                    let recreated = worktree_service.recreate_worktree(&worktree, request.force)?;
                    paths.push(recreated.path);
                }
            } else {
                if request.recreate
                    && !worktree_service
                        .bookmark_exists_in_repo(repo.repo_path.as_str(), &branch_name)?
                {
                    if request.all_repos {
                        warnings.push(format!(
                            "Skipping create for {} (missing branch/bookmark).",
                            repo.repo_path
                        ));
                        continue;
                    }
                    return Err(TrackError::BookmarkNotFound {
                        bookmark: branch_name.clone(),
                        repo_path: repo.repo_path.clone(),
                    });
                }

                let created = match worktree_service.add_worktree(
                    task_id,
                    &repo.repo_path,
                    None,
                    task.ticket_id.as_deref(),
                    Some(todo.id),
                    false,
                ) {
                    Ok(worktree) => worktree,
                    Err(TrackError::BookmarkExists(_)) => worktree_service.add_existing_worktree(
                        task_id,
                        &repo.repo_path,
                        &branch_name,
                        Some(todo.id),
                        false,
                        None,
                    )?,
                    Err(err) => return Err(err),
                };

                paths.push(created.path);
            }
        }

        if paths.is_empty() {
            return Err(TrackError::NoWorkspacePathsAvailable);
        }

        Ok(TodoWorkspaceOutcome { paths, warnings })
    }
}

fn resolve_current_repo(repos: &[TaskRepo]) -> Result<TaskRepo> {
    let current_path = std::env::current_dir()
        .and_then(|path| path.canonicalize())
        .map_err(|e| TrackError::PathResolutionFailed(e.to_string()))?;

    let mut matching: Vec<(TaskRepo, PathBuf)> = repos
        .iter()
        .cloned()
        .filter_map(|repo| {
            let repo_path = PathBuf::from(&repo.repo_path);
            let repo_path = repo_path.canonicalize().ok()?;
            if current_path.starts_with(&repo_path) {
                Some((repo, repo_path))
            } else {
                None
            }
        })
        .collect();

    matching.sort_by_key(|(_, repo_path)| repo_path.to_string_lossy().len());
    matching
        .pop()
        .map(|(repo, _)| repo)
        .ok_or(TrackError::CurrentDirectoryNotRegistered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::TaskService;

    #[test]
    fn workspace_requires_registered_repos() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Todo", false)
            .unwrap();

        let result = TodoWorkspaceUseCase::new(&db).execute(
            task.id,
            1,
            TodoWorkspaceRequest {
                recreate: false,
                force: false,
                all_repos: false,
            },
        );
        assert!(matches!(result, Err(TrackError::NoRepositoriesRegistered)));
    }
}
