use crate::cli::handlers::CommandCtx;
use crate::models::TodoAddOptions;
use crate::services::{TaskService, TodoService, WorktreeService};
use crate::use_cases::{ArchiveTaskUseCase, CreateTodayTaskUseCase, GetTaskInfoUseCase};
use crate::utils::{Result, TrackError};
use chrono::Local;
use prettytable::{format, Cell, Row, Table};
use std::io::{self, Write};

pub fn handle_new(
    ctx: &CommandCtx,
    name: &str,
    description: Option<&str>,
    ticket: Option<&str>,
    ticket_url: Option<&str>,
    template: Option<&str>,
) -> Result<()> {
    let task_service = TaskService::new(ctx.db);
    let task = task_service.create_task(name, description, ticket, ticket_url)?;

    println!("Created task #{}: {}", task.id, task.name);
    if let Some(ticket_id) = &task.ticket_id {
        print!("Ticket: {}", ticket_id);
        if let Some(url) = &task.ticket_url {
            print!(" ({})", url);
        }
        println!();
    }
    println!("Switched to task #{}", task.id);

    // If template is specified, copy TODOs from template task
    if let Some(template_ref) = template {
        let template_task_id = task_service.resolve_task_id(template_ref)?;
        let template_task = task_service.get_task(template_task_id)?;

        let todo_service = TodoService::new(ctx.db);
        let template_todos = todo_service.list_todos(template_task_id)?;

        if template_todos.is_empty() {
            println!(
                "Warning: Template task '{}' has no TODOs",
                template_task.name
            );
        } else {
            println!(
                "\nCopying {} TODOs from template task '{}'...",
                template_todos.len(),
                template_task.name
            );

            for template_todo in &template_todos {
                todo_service.add_todo(
                    task.id,
                    &template_todo.content,
                    TodoAddOptions {
                        worktree_requested: template_todo.worktree_requested,
                        requires_workspace: template_todo.requires_workspace,
                    },
                )?;
            }

            println!("Successfully copied {} TODOs", template_todos.len());
        }
    }

    Ok(())
}

pub fn handle_list(ctx: &CommandCtx, include_archived: bool) -> Result<()> {
    let task_service = TaskService::new(ctx.db);
    let tasks = task_service.list_tasks(include_archived)?;
    let current_task_id = ctx.db.get_current_task_id()?;

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(Row::new(vec![
        Cell::new(""),
        Cell::new("ID"),
        Cell::new("Ticket"),
        Cell::new("Name"),
        Cell::new("Status"),
        Cell::new("Created"),
    ]));

    for task in tasks {
        let is_current = current_task_id == Some(task.id);
        let marker = if is_current { "*" } else { " " };
        let ticket = task.ticket_id.as_deref().unwrap_or("-");
        let created = task
            .created_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S");

        table.add_row(Row::new(vec![
            Cell::new(marker),
            Cell::new(&task.id.to_string()),
            Cell::new(ticket),
            Cell::new(&task.name),
            Cell::new(task.status.as_str()),
            Cell::new(&created.to_string()),
        ]));
    }

    table.printstd();
    Ok(())
}

pub fn handle_switch(ctx: &CommandCtx, task_ref: &str) -> Result<()> {
    let task_service = TaskService::new(ctx.db);

    // Check if the user wants to switch to today's task
    if task_ref.to_lowercase() == "today" {
        let task = CreateTodayTaskUseCase::new(ctx.db).get_or_create()?;
        // Update the current task context
        task_service.switch_task(task.id)?;
        println!("Switched to today's task: {}", task.name);
        return Ok(());
    }

    // Normal task switching
    let task_id = task_service.resolve_task_id(task_ref)?;
    let task = task_service.switch_task(task_id)?;

    println!("Switched to task #{}: {}", task.id, task.name);
    Ok(())
}

pub fn handle_info(
    ctx: &CommandCtx,
    task_ref: Option<String>,
    json: bool,
    all_scraps: bool,
) -> Result<()> {
    let task_service = TaskService::new(ctx.db);
    let task_id = match task_ref {
        Some(ref t_ref) => task_service.resolve_task_id(t_ref)?,
        None => ctx
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?,
    };

    let info = GetTaskInfoUseCase::new(ctx.db);
    let snapshot = info.load(task_id)?;
    let worktree_service = WorktreeService::new(ctx.db);

    if json {
        let output = info.to_cli_json(&snapshot)?;
        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| TrackError::SerializationFailed(e.to_string()))?;
        println!("{json}");
        return Ok(());
    }

    let task = &snapshot.task;
    let todos = &snapshot.todos;
    let links = &snapshot.links;
    let scraps = &snapshot.scraps;
    let worktrees = &snapshot.worktrees;
    let repos = &snapshot.repos;
    let base_branch = GetTaskInfoUseCase::base_bookmark(&snapshot);

    println!("# Task #{}: {}", task.id, task.name);
    println!();

    let created = task
        .created_at
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S");
    println!("**Created:** {created}");

    if let Some(ticket_id) = &task.ticket_id {
        if let Some(url) = &task.ticket_url {
            println!("**Ticket:** [{}]({url})", ticket_id);
        } else {
            println!("**Ticket:** {ticket_id}");
        }
    }

    println!("**Base Bookmark:** `{base_branch}`");
    println!();

    // Description
    if let Some(desc) = &task.description {
        println!("## Description");
        println!();
        println!("{}", desc);
        println!();
    }

    // TODOs
    if !todos.is_empty() {
        println!("## TODOs");
        println!();
        for todo in todos {
            let marker = match todo.status.as_str() {
                "done" => "x",
                "cancelled" => " ",
                _ => " ",
            };
            let status_indicator = match todo.status.as_str() {
                "cancelled" => " ~~",
                _ => "",
            };
            let status_end = match todo.status.as_str() {
                "cancelled" => "~~",
                _ => "",
            };
            if let Some(completed_at) = todo.completed_at {
                let done_time = completed_at.with_timezone(&Local).format("%Y-%m-%d %H:%M");
                println!(
                    "- [{}] **[{}]**{} {}{} (done: {})",
                    marker, todo.task_index, status_indicator, todo.content, status_end, done_time
                );
            } else {
                println!(
                    "- [{}] **[{}]**{} {}{}",
                    marker, todo.task_index, status_indicator, todo.content, status_end
                );
            }

            // Find and display worktree for this TODO
            for worktree in worktrees {
                if worktree.todo_id == Some(todo.id) {
                    println!("  - **Workspace:**");
                    println!("    - **Path:** `{}`", worktree.path);
                    println!("    - **Bookmark:** `{}`", worktree.branch);

                    let repo_links = worktree_service.list_repo_links(worktree.id)?;
                    if !repo_links.is_empty() {
                        println!("    - **Repository Links:**");
                        for link in repo_links {
                            println!("      - {}: {}", link.kind, link.url);
                        }
                    }
                }
            }
        }
        println!();
    }

    // Links
    if !links.is_empty() {
        println!("## Links");
        println!();
        for link in links {
            println!("- [{}]({})", link.title, link.url);
        }
        println!();
    }

    // Repositories
    if !repos.is_empty() {
        println!("## Repositories");
        println!();
        for repo in repos {
            print!("- `{}`", repo.repo_path);

            // Display base branch and commit hash if available
            if let Some(ref base_branch) = repo.base_branch {
                if let Some(ref base_hash) = repo.base_commit_hash {
                    // Show both branch and short hash
                    let short_hash = &base_hash[..std::cmp::min(8, base_hash.len())];
                    print!(" (base: {} @ {})", base_branch, short_hash);
                } else {
                    // Show only branch
                    print!(" (base: {})", base_branch);
                }
            } else if let Some(ref base_hash) = repo.base_commit_hash {
                // Show only hash
                let short_hash = &base_hash[..std::cmp::min(8, base_hash.len())];
                print!(" (base: {})", short_hash);
            }

            println!();
        }
        println!();
    }

    // Scraps
    if !scraps.is_empty() {
        if all_scraps {
            println!("## Scraps");
        } else {
            println!("## Recent Scraps");
        }
        println!();

        let count = if all_scraps { scraps.len() } else { 5 };

        for scrap in scraps.iter().take(count) {
            let timestamp = scrap.created_at.with_timezone(&Local).format("%H:%M");
            println!("### [{}]", timestamp);
            println!();
            // Wrap content in blockquote to prevent markdown heading conflicts
            for line in scrap.content.lines() {
                if line.is_empty() {
                    println!(">");
                } else {
                    println!("> {}", line);
                }
            }
            println!();
        }
        println!();
    }

    // Worktrees (only those not associated with a TODO, e.g., base worktrees)
    let orphan_worktrees = GetTaskInfoUseCase::orphan_worktrees(&snapshot);

    if !orphan_worktrees.is_empty() {
        println!("## Workspaces");
        println!();
        for worktree in orphan_worktrees {
            println!("### Workspace #{}", worktree.id);
            println!();
            println!("- **Path:** `{}`", worktree.path);
            println!("- **Bookmark:** `{}`", worktree.branch);

            let repo_links = worktree_service.list_repo_links(worktree.id)?;
            if !repo_links.is_empty() {
                println!("- **Repository Links:**");
                for link in repo_links {
                    println!("  - {}: {}", link.kind, link.url);
                }
            }
            println!();
        }
    }

    Ok(())
}

pub fn handle_desc(ctx: &CommandCtx, description: Option<&str>, task: Option<i64>) -> Result<()> {
    let task_id = match task {
        Some(id) => id,
        None => ctx
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?,
    };

    let task_service = TaskService::new(ctx.db);

    match description {
        Some(desc) => {
            // Set mode
            task_service.set_description(task_id, desc)?;
            println!("Updated description for task #{}", task_id);
        }
        None => {
            // View mode
            let task = task_service.get_task(task_id)?;
            println!("=== Task #{}: {} ===", task.id, task.name);
            println!();

            if let Some(desc) = &task.description {
                println!("Description:");
                println!("  {}", desc);
            } else {
                println!("No description set. Use 'track desc <text>' to add one.");
            }
        }
    }

    Ok(())
}

pub fn handle_ticket(
    ctx: &CommandCtx,
    ticket_id: &str,
    url: &str,
    task: Option<i64>,
) -> Result<()> {
    let task_id = match task {
        Some(id) => id,
        None => ctx
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?,
    };

    let task_service = TaskService::new(ctx.db);
    task_service.link_ticket(task_id, ticket_id, url)?;

    println!("Linked ticket {} to task #{}", ticket_id, task_id);
    println!("URL: {}", url);

    Ok(())
}

pub fn handle_archive(ctx: &CommandCtx, task_ref: Option<&str>, force: bool) -> Result<()> {
    let use_case = ArchiveTaskUseCase::new(ctx.db);
    let task_id = use_case.resolve_task_id(task_ref)?;

    let outcome = match use_case.execute(task_id, force) {
        Ok(outcome) => outcome,
        Err(TrackError::UncommittedWorkspaces(workspaces)) => {
            println!("WARNING: The following workspaces have uncommitted changes:");
            for line in &workspaces {
                println!("  {}", line);
            }
            println!();
            print!("Archive and remove workspaces anyway? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Cancelled.");
                return Ok(());
            }

            use_case.execute(task_id, true)?
        }
        Err(TrackError::JjTaskNotCompleted { slug, workspaces }) => {
            println!("WARNING: jj-task workspace '{slug}' is not marked done.",);
            println!("  Merge your PR with the $jj skill, then run: jj-task done {slug}");
            for path in &workspaces {
                println!("  {}", path);
            }
            println!();
            print!("Archive the track task anyway (jj-task map unchanged)? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Cancelled.");
                return Ok(());
            }

            use_case.execute(task_id, true)?
        }
        Err(err) => return Err(err),
    };

    if !outcome.removed_workspaces.is_empty() {
        println!("Cleaning up workspaces...");
        for (id, path) in &outcome.removed_workspaces {
            println!("  Removed workspace #{}: {}", id, path);
        }
    }

    for err in &outcome.workspace_errors {
        eprintln!("  Error removing workspace: {}", err);
    }

    println!("Archived task #{}: {}", outcome.task.id, outcome.task.name);

    Ok(())
}
