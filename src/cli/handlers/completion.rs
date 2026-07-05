use crate::cli::handlers::CommandCtx;
use crate::cli::CompletionType;
use crate::models::TodoStatus;
use crate::services::{LinkService, RepoService, TaskService, TodoService};
use crate::utils::Result;

pub fn handle_completion(
    _ctx: &CommandCtx,
    shell: clap_complete::Shell,
    dynamic: bool,
) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io;

    if dynamic {
        // Output dynamic completion script
        let script = match shell {
            clap_complete::Shell::Bash => {
                include_str!("../../../completions/track.bash.dynamic")
            }
            clap_complete::Shell::Zsh => {
                include_str!("../../../completions/_track.dynamic")
            }
            _ => {
                eprintln!("Dynamic completions are only available for bash and zsh.");
                eprintln!("Falling back to static completions for {:?}.", shell);
                let mut cmd = crate::cli::Cli::command();
                let bin_name = cmd.get_name().to_string();
                generate(shell, &mut cmd, bin_name, &mut io::stdout());
                return Ok(());
            }
        };
        print!("{}", script);
    } else {
        // Generate static completion using clap_complete
        let mut cmd = crate::cli::Cli::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut io::stdout());
    }

    Ok(())
}

pub fn handle_complete(ctx: &CommandCtx, completion_type: CompletionType) -> Result<()> {
    match completion_type {
        CompletionType::Tasks => {
            // Output task IDs and names for 'track switch'
            let task_service = TaskService::new(ctx.db);
            let tasks = task_service.list_tasks(false)?; // Don't include archived

            for task in tasks {
                // Format: ID:Name (or ID:Ticket:Name if ticket exists)
                if let Some(ticket) = &task.ticket_id {
                    println!("{}:{}:{}", task.id, ticket, task.name);
                } else {
                    println!("{}:{}", task.id, task.name);
                }
            }
        }
        CompletionType::Todos => {
            // Output TODO IDs and content for current task
            let current_task_id = ctx.db.get_current_task_id()?;
            if let Some(task_id) = current_task_id {
                let todo_service = TodoService::new(ctx.db);
                let todos = todo_service.list_todos(task_id)?;

                for todo in todos {
                    // Only show pending todos
                    if todo.status == TodoStatus::Pending {
                        // Format: ID:Content
                        println!("{}:{}", todo.task_index, todo.content);
                    }
                }
            }
        }
        CompletionType::Links => {
            // Output link IDs and URLs for current task
            let current_task_id = ctx.db.get_current_task_id()?;
            if let Some(task_id) = current_task_id {
                let link_service = LinkService::new(ctx.db);
                let links = link_service.list_links(task_id)?;

                for link in links {
                    // Format: ID:Title:URL
                    println!("{}:{}:{}", link.task_index, link.title, link.url);
                }
            }
        }
        CompletionType::Repos => {
            // Output repo IDs and paths for current task
            let current_task_id = ctx.db.get_current_task_id()?;
            if let Some(task_id) = current_task_id {
                let repo_service = RepoService::new(ctx.db);
                let repos = repo_service.list_repos(task_id)?;

                for repo in repos {
                    // Format: ID:Path
                    println!("{}:{}", repo.task_index, repo.repo_path);
                }
            }
        }
    }

    Ok(())
}
