use crate::cli::handlers::CommandCtx;
use crate::services::agent_context::build_agent_extensions;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::use_cases::{ArchiveTaskUseCase, CreateTodayTaskUseCase};
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
                    template_todo.worktree_requested,
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
    let current_task_id = match task_ref {
        Some(ref t_ref) => task_service.resolve_task_id(t_ref)?,
        None => ctx
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?,
    };

    let task = task_service.get_task(current_task_id)?;

    let todo_service = TodoService::new(ctx.db);
    let todos = todo_service.list_todos(current_task_id)?;

    let link_service = LinkService::new(ctx.db);
    let links = link_service.list_links(current_task_id)?;

    let scrap_service = ScrapService::new(ctx.db);
    let scraps = scrap_service.list_scraps(current_task_id)?;

    let worktree_service = WorktreeService::new(ctx.db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let repo_service = RepoService::new(ctx.db);
    let repos = repo_service.list_repos(current_task_id)?;

    if json {
        // Build todos with worktree_branch
        let mut todos_json = Vec::new();
        for todo in &todos {
            let mut todo_val = serde_json::to_value(todo).unwrap_or(serde_json::Value::Null);
            if let Some(obj) = todo_val.as_object_mut() {
                // Determine worktree branch
                let worktree_branch =
                    if let Some(wt) = worktrees.iter().find(|wt| wt.todo_id == Some(todo.id)) {
                        // Use existing worktree branch
                        Some(wt.branch.clone())
                    } else if todo.worktree_requested {
                        // Calculate expected branch name
                        worktree_service
                            .get_todo_branch_name(
                                current_task_id,
                                task.ticket_id.as_deref(),
                                todo.task_index, // Note: this is todo_id in JSON but task_index in struct
                            )
                            .ok()
                    } else {
                        None
                    };

                obj.insert(
                    "worktree_branch".to_string(),
                    serde_json::to_value(&worktree_branch).unwrap_or(serde_json::Value::Null),
                );
            }
            todos_json.push(todo_val);
        }

        let mut worktrees_json = Vec::new();
        for wt in &worktrees {
            let mut wt_val = serde_json::to_value(wt).unwrap_or(serde_json::Value::Null);
            if let Some(obj) = wt_val.as_object_mut() {
                obj.remove("id");
                obj.remove("is_base");
                obj.remove("task_id");

                let task_scoped_id = wt
                    .todo_id
                    .and_then(|id| todos.iter().find(|t| t.id == id).map(|t| t.task_index));
                obj.insert(
                    "todo_id".to_string(),
                    serde_json::to_value(task_scoped_id).unwrap_or(serde_json::Value::Null),
                );
            }
            worktrees_json.push(wt_val);
        }

        // Add pending worktrees (requested in TODOs but not yet created via sync)
        for todo in &todos {
            if todo.worktree_requested && !worktrees.iter().any(|wt| wt.todo_id == Some(todo.id)) {
                let branch_name = worktree_service
                    .get_todo_branch_name(
                        current_task_id,
                        task.ticket_id.as_deref(),
                        todo.task_index,
                    )
                    .ok();

                // Create a virtual worktree object for the pending state
                let pending_wt = serde_json::json!({
                    "todo_id": todo.task_index,
                    "branch": branch_name,
                    "status": "requested",
                    "path": null,
                    "created_at": null,
                    "base_repo": null
                });
                worktrees_json.push(pending_wt);
            }
        }

        let mut output = serde_json::json!({
            "task": task,
            "todos": todos_json,
            "links": links,
            "scraps": scraps,
            "worktrees": worktrees_json,
            "repos": repos,
        });

        let agent = build_agent_extensions(&task, &todos, &worktrees, &repos, &worktree_service);
        if let Some(obj) = output.as_object_mut() {
            for (key, value) in agent.as_object().into_iter().flatten() {
                obj.insert(key.clone(), value.clone());
            }
        }

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return Ok(());
    }

    // Task header
    println!("# Task #{}: {}", task.id, task.name);
    println!();

    // Metadata section
    let created = task
        .created_at
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S");
    println!("**Created:** {}", created);

    if let Some(ticket_id) = &task.ticket_id {
        if let Some(url) = &task.ticket_url {
            println!("**Ticket:** [{}]({})", ticket_id, url);
        } else {
            println!("**Ticket:** {}", ticket_id);
        }
    }

    // Display base bookmark (the task bookmark that serves as integration target for TODO workspaces)
    let base_branch = if let Some(base_wt) = worktrees.iter().find(|wt| wt.is_base) {
        // If base workspace exists, use its bookmark
        base_wt.branch.clone()
    } else {
        // Otherwise, calculate the task branch name (same logic as in handle_sync)
        if let Some(ticket_id) = &task.ticket_id {
            format!("task/{}", ticket_id)
        } else {
            format!("task/task-{}", task.id)
        }
    };

    println!("**Base Bookmark:** `{}`", base_branch);

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
        for todo in &todos {
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
            for worktree in &worktrees {
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
        for repo in &repos {
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
    let orphan_worktrees: Vec<_> = worktrees.iter().filter(|wt| wt.todo_id.is_none()).collect();

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

pub fn handle_archive(ctx: &CommandCtx, task_ref: Option<&str>) -> Result<()> {
    let use_case = ArchiveTaskUseCase::new(ctx.db);
    let task_id = use_case.resolve_task_id(task_ref)?;

    let outcome = match use_case.execute(task_id, false) {
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
