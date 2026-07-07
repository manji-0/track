use crate::cli::handlers::CommandCtx;
use crate::cli::TodoCommands;
use crate::models::{TodoAction, TodoAddOptions, TodoStatus};
use crate::services::TodoService;
use crate::use_cases::{
    ApplyTodoActionUseCase, CompleteTodoUseCase, TodoWorkspaceRequest, TodoWorkspaceUseCase,
};
use crate::utils::{Result, TrackError};
use prettytable::{format, Cell, Row, Table};
use std::io::{self, Write};

pub fn handle_todo(ctx: &CommandCtx, command: TodoCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let todo_service = TodoService::new(ctx.db);

    match command {
        TodoCommands::Add {
            text,
            worktree,
            no_workspace,
        } => {
            if worktree {
                return Err(TrackError::WorktreeFlagRemoved);
            }
            let options = TodoAddOptions::from_flags(false, no_workspace);
            let todo = todo_service.add_todo(current_task_id, &text, options)?;
            println!("Added TODO #{}: {}", todo.task_index, todo.content);

            if no_workspace {
                println!("No jj-task/git workspace required for this TODO");
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
            let todo = todo_service.get_todo_by_index(current_task_id, id)?;
            if status == TodoStatus::PENDING {
                todo_service.update_status(todo.id, &status)?;
            } else {
                let action = TodoAction::from_cli_update_status(&status)?;
                ApplyTodoActionUseCase::new(ctx.db).execute(current_task_id, id, action)?;
            }
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
            let outcome = TodoWorkspaceUseCase::new(ctx.db).execute(
                current_task_id,
                id,
                TodoWorkspaceRequest {
                    recreate,
                    force,
                    all_repos: all,
                },
            )?;

            for warning in &outcome.warnings {
                eprintln!("{warning}");
            }

            if all {
                for path in outcome.paths {
                    println!("{path}");
                }
            } else {
                println!("{}", outcome.paths[0]);
                if outcome.paths.len() > 1 {
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
