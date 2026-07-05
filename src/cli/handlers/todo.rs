use crate::cli::handlers::CommandCtx;
use crate::cli::TodoCommands;
use crate::models::TaskRepo;
use crate::services::{RepoService, TaskService, TodoService, WorktreeService};
use crate::use_cases::CompleteTodoUseCase;
use crate::utils::{Result, TrackError};
use prettytable::{format, Cell, Row, Table};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn handle_todo(ctx: &CommandCtx, command: TodoCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let todo_service = TodoService::new(ctx.db);

    match command {
        TodoCommands::Add { text, worktree } => {
            let todo = todo_service.add_todo(current_task_id, &text, worktree)?;
            println!("Added TODO #{}: {}", todo.task_index, todo.content);

            if worktree {
                println!("Workspace creation scheduled for 'track sync'");
            }
        }
        TodoCommands::List => {
            let todos = todo_service.list_todos(current_task_id)?;
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.set_titles(Row::new(vec![
                Cell::new("ID"),
                Cell::new("Status"),
                Cell::new("Content"),
            ]));

            for todo in todos {
                table.add_row(Row::new(vec![
                    Cell::new(&todo.task_index.to_string()),
                    Cell::new(todo.status.as_str()),
                    Cell::new(&todo.content),
                ]));
            }

            table.printstd();
        }
        TodoCommands::Update { id, status } => {
            // Resolve task_index to internal ID
            let todo = todo_service.get_todo_by_index(current_task_id, id)?;
            todo_service.update_status(todo.id, &status)?;
            println!("Updated TODO #{} status to '{}'", id, status);
        }
        TodoCommands::Done { id } => {
            let outcome = CompleteTodoUseCase::new(ctx.db).execute(current_task_id, id)?;
            if let Some(branch) = outcome.merged_bookmark {
                println!(
                    "Rebased and removed workspace for TODO #{} (bookmark: {}).",
                    id, branch
                );
            }
            println!("Marked TODO #{} as done.", id);
        }
        TodoCommands::Workspace {
            id,
            recreate,
            force,
            all,
        } => {
            let todo = todo_service.get_todo_by_index(current_task_id, id)?;
            let worktree_service = WorktreeService::new(ctx.db);
            let repo_service = RepoService::new(ctx.db);
            let repos = repo_service.list_repos(current_task_id)?;

            if repos.is_empty() {
                return Err(TrackError::Other(
                    "No repositories registered for this task.".to_string(),
                ));
            }

            let target_repos = if all {
                repos
            } else {
                let current_path = std::env::current_dir()
                    .and_then(|path| path.canonicalize())
                    .map_err(|e| {
                        TrackError::Other(format!("Failed to resolve current path: {}", e))
                    })?;

                let mut matching: Vec<(TaskRepo, PathBuf)> = repos
                    .into_iter()
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
                let repo = matching.pop().map(|(repo, _)| repo).ok_or_else(|| {
                    TrackError::Other(
                        "Current directory is not a registered repo for this task.".to_string(),
                    )
                })?;
                vec![repo]
            };

            let worktrees = worktree_service.list_worktrees(current_task_id)?;
            let task_service = TaskService::new(ctx.db);
            let task = task_service.get_task(current_task_id)?;
            let branch_name = worktree_service.get_todo_branch_name(
                current_task_id,
                task.ticket_id.as_deref(),
                todo.task_index,
            )?;

            let mut output_paths: Vec<String> = Vec::new();

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
                    if !recreate {
                        output_paths
                            .extend(todo_worktrees.iter().map(|worktree| worktree.path.clone()));
                        continue;
                    }

                    if !force {
                        for worktree in &todo_worktrees {
                            if Path::new(&worktree.path).exists()
                                && worktree_service.has_uncommitted_changes(&worktree.path)?
                            {
                                return Err(TrackError::Other(format!(
                                        "Workspace {} has uncommitted changes. Use --force to recreate.",
                                        worktree.path
                                    )));
                            }
                        }
                    }

                    for worktree in todo_worktrees.drain(..) {
                        if !worktree_service
                            .bookmark_exists_in_repo(repo.repo_path.as_str(), &worktree.branch)?
                        {
                            if Path::new(&worktree.path).exists() {
                                output_paths.push(worktree.path.clone());
                            }
                            if all {
                                eprintln!(
                                    "Skipping recreate for {} (missing branch/bookmark).",
                                    worktree.path
                                );
                                continue;
                            }
                            return Err(TrackError::Other(format!(
                                "Bookmark {} not found in {}.",
                                worktree.branch, repo.repo_path
                            )));
                        }

                        let recreated = worktree_service.recreate_worktree(&worktree, force)?;
                        output_paths.push(recreated.path);
                    }
                } else {
                    if recreate
                        && !worktree_service
                            .bookmark_exists_in_repo(repo.repo_path.as_str(), &branch_name)?
                    {
                        if all {
                            eprintln!(
                                "Skipping create for {} (missing branch/bookmark).",
                                repo.repo_path
                            );
                            continue;
                        }
                        return Err(TrackError::Other(format!(
                            "Bookmark {} not found in {}.",
                            branch_name, repo.repo_path
                        )));
                    }

                    let created = match worktree_service.add_worktree(
                        current_task_id,
                        &repo.repo_path,
                        None,
                        task.ticket_id.as_deref(),
                        Some(todo.id),
                        false,
                    ) {
                        Ok(worktree) => worktree,
                        Err(TrackError::BookmarkExists(_)) => worktree_service
                            .add_existing_worktree(
                                current_task_id,
                                &repo.repo_path,
                                &branch_name,
                                Some(todo.id),
                                false,
                                None,
                            )?,
                        Err(e) => return Err(e),
                    };

                    output_paths.push(created.path);
                }
            }

            if output_paths.is_empty() {
                return Err(TrackError::Other(
                    "No workspace paths available for this TODO.".to_string(),
                ));
            }

            if all {
                for path in output_paths {
                    println!("{}", path);
                }
            } else {
                println!("{}", output_paths[0]);
                if output_paths.len() > 1 {
                    eprintln!(
                        "Multiple workspaces exist for TODO #{}. Using first path.",
                        id
                    );
                }
            }
        }
        TodoCommands::Delete { id, force } => {
            // Resolve task_index to internal ID
            let todo = todo_service.get_todo_by_index(current_task_id, id)?;

            if !force {
                print!("Delete TODO #{}: \"{}\"? [y/N]: ", id, todo.content);
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            todo_service.delete_todo(todo.id)?;
            println!("Deleted TODO #{}", id);
        }
        TodoCommands::Next { id } => {
            todo_service.move_to_next(current_task_id, id)?;
            println!("Moved TODO #{} to the front (next todo to work on)", id);
        }
    }

    Ok(())
}
