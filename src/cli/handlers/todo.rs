use crate::cli::handlers::confirm::confirm_from_tty;
use crate::cli::handlers::json_out::{emit_mutation, MutationKind};
use crate::cli::handlers::CommandCtx;
use crate::cli::TodoCommands;
use crate::models::{TodoAction, TodoAddOptions};
use crate::services::TodoService;
use crate::use_cases::{
    ApplyTodoActionUseCase, CompleteTodoUseCase, DeleteTodoStep, DeleteTodoUseCase,
    TodoWorkspaceRequest, TodoWorkspaceUseCase,
};
use crate::utils::{Result, TrackError};
use prettytable::{format, Cell, Row, Table};

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
            json,
        } => {
            if worktree {
                return Err(TrackError::WorktreeFlagRemoved);
            }
            let options = TodoAddOptions::from_flags(false, no_workspace);
            let todo = todo_service.add_todo(current_task_id, &text, options)?;
            emit_mutation(
                ctx,
                json,
                MutationKind::TodoAdd,
                Some(todo.task_index),
                Some(current_task_id),
                || {
                    println!("Added TODO #{}: {}", todo.task_index, todo.content);
                    if no_workspace {
                        println!("No jj-task/git workspace required for this TODO");
                    }
                },
            )
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
            Ok(())
        }
        TodoCommands::Update { id, status, json } => {
            let action = TodoAction::from_cli_update_status(status)?;
            ApplyTodoActionUseCase::new(ctx.db).execute(current_task_id, id, action)?;
            emit_mutation(
                ctx,
                json,
                MutationKind::TodoUpdate,
                Some(id),
                Some(current_task_id),
                || println!("Updated TODO #{id} status to '{}'", status.as_str()),
            )
        }
        TodoCommands::Done { id, json } => {
            let outcome = CompleteTodoUseCase::new(ctx.db).execute(current_task_id, id)?;
            emit_mutation(
                ctx,
                json,
                MutationKind::TodoDone,
                Some(id),
                Some(current_task_id),
                || {
                    if let Some(branch) = outcome.merged_bookmark {
                        println!(
                            "Rebased and removed workspace for TODO #{id} (bookmark: {branch})."
                        );
                    }
                    println!("Marked TODO #{id} as done.");
                },
            )
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
            Ok(())
        }
        TodoCommands::Delete { id, force, json } => {
            let use_case = DeleteTodoUseCase::new(ctx.db);
            let outcome = match use_case.run(current_task_id, id, force)? {
                DeleteTodoStep::Completed(outcome) => outcome,
                DeleteTodoStep::NeedsConfirmation(prompt) => {
                    let view = prompt.view();
                    if !confirm_from_tty(&view.prompt, &view.non_tty_hint)? {
                        println!("Cancelled.");
                        return Ok(());
                    }

                    use_case.confirm_and_run(current_task_id, id)?
                }
            };
            emit_mutation(
                ctx,
                json,
                MutationKind::TodoDelete,
                Some(outcome.task_index),
                Some(current_task_id),
                || println!("{}", outcome.completion_view().summary),
            )
        }
        TodoCommands::Next { id, json } => {
            todo_service.move_to_next(current_task_id, id)?;
            emit_mutation(
                ctx,
                json,
                MutationKind::TodoNext,
                Some(id),
                Some(current_task_id),
                || println!("Moved TODO #{id} to the front (next todo to work on)"),
            )
        }
    }
}
