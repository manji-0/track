use crate::cli::{
    AliasCommands, Commands, CompletionType, LinkCommands, RepoCommands, ScrapCommands,
    TodoCommands,
};
use crate::db::Database;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::utils::{Result, TrackError};
use chrono::Local;
use prettytable::{format, Cell, Row, Table};
use std::io::{self, Write};

pub struct CommandHandler {
    db: Database,
}

impl CommandHandler {
    pub fn new() -> Result<Self> {
        let db = Database::new()?;
        Ok(Self { db })
    }

    #[allow(dead_code)]
    pub fn from_db(db: Database) -> Self {
        Self { db }
    }

    /// Returns a reference to the database instance.
    /// This is primarily used for testing.
    #[allow(dead_code)]
    pub fn get_db(&self) -> &Database {
        &self.db
    }

    pub fn handle(&self, command: Commands) -> Result<()> {
        match command {
            Commands::New {
                name,
                description,
                ticket,
                ticket_url,
                template,
            } => self.handle_new(
                &name,
                description.as_deref(),
                ticket.as_deref(),
                ticket_url.as_deref(),
                template.as_deref(),
            ),
            Commands::List { all } => self.handle_list(all),
            Commands::Switch { task_ref } => self.handle_switch(&task_ref),
            Commands::Status { id, json, all } => self.handle_info(id, json, all),
            Commands::Desc { description, task } => self.handle_desc(description.as_deref(), task),
            Commands::Ticket {
                ticket_id,
                url,
                task,
            } => self.handle_ticket(&ticket_id, &url, task),
            Commands::Archive { task_ref } => self.handle_archive(task_ref.as_deref()),
            Commands::Todo(cmd) => self.handle_todo(cmd),
            Commands::Link(cmd) => self.handle_link(cmd),
            Commands::Scrap(cmd) => self.handle_scrap(cmd),
            Commands::Sync => self.handle_sync(),
            Commands::Repo(cmd) => self.handle_repo(cmd),
            Commands::Alias(cmd) => self.handle_alias(cmd),
            Commands::LlmHelp => self.handle_llm_help(),
            Commands::Completion { shell, dynamic } => self.handle_completion(shell, dynamic),
            Commands::Complete { completion_type } => self.handle_complete(completion_type),
            // Webui is handled directly in main.rs with async runtime
            Commands::Webui { .. } => unreachable!("Webui command is handled in main.rs"),
        }
    }

    fn handle_new(
        &self,
        name: &str,
        description: Option<&str>,
        ticket: Option<&str>,
        ticket_url: Option<&str>,
        template: Option<&str>,
    ) -> Result<()> {
        let task_service = TaskService::new(&self.db);
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

            let todo_service = TodoService::new(&self.db);
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

    fn handle_list(&self, include_archived: bool) -> Result<()> {
        let task_service = TaskService::new(&self.db);
        let tasks = task_service.list_tasks(include_archived)?;
        let current_task_id = self.db.get_current_task_id()?;

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
                Cell::new(&task.status),
                Cell::new(&created.to_string()),
            ]));
        }

        table.printstd();
        Ok(())
    }

    fn handle_switch(&self, task_ref: &str) -> Result<()> {
        let task_service = TaskService::new(&self.db);
        let task_id = task_service.resolve_task_id(task_ref)?;
        let task = task_service.switch_task(task_id)?;

        println!("Switched to task #{}: {}", task.id, task.name);
        Ok(())
    }

    fn handle_info(&self, task_ref: Option<String>, json: bool, all_scraps: bool) -> Result<()> {
        let task_service = TaskService::new(&self.db);
        let current_task_id = match task_ref {
            Some(ref t_ref) => task_service.resolve_task_id(t_ref)?,
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask)?,
        };

        let task = task_service.get_task(current_task_id)?;

        let todo_service = TodoService::new(&self.db);
        let todos = todo_service.list_todos(current_task_id)?;

        let link_service = LinkService::new(&self.db);
        let links = link_service.list_links(current_task_id)?;

        let scrap_service = ScrapService::new(&self.db);
        let scraps = scrap_service.list_scraps(current_task_id)?;

        let worktree_service = WorktreeService::new(&self.db);
        let worktrees = worktree_service.list_worktrees(current_task_id)?;

        let repo_service = RepoService::new(&self.db);
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
                if todo.worktree_requested
                    && !worktrees.iter().any(|wt| wt.todo_id == Some(todo.id))
                {
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

            let output = serde_json::json!({
                "task": task,
                "todos": todos_json,
                "links": links,
                "scraps": scraps,
                "worktrees": worktrees_json,
                "repos": repos,
            });

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

        // Display base branch (the task branch that serves as merge target for TODO worktrees)
        let base_branch = if let Some(base_wt) = worktrees.iter().find(|wt| wt.is_base) {
            // If base worktree exists, use its branch
            base_wt.branch.clone()
        } else {
            // Otherwise, calculate the task branch name (same logic as in handle_sync)
            if let Some(ticket_id) = &task.ticket_id {
                format!("task/{}", ticket_id)
            } else {
                format!("task/task-{}", task.id)
            }
        };

        println!("**Base Branch:** `{}`", base_branch);

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
                        marker,
                        todo.task_index,
                        status_indicator,
                        todo.content,
                        status_end,
                        done_time
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
                        println!("  - **Worktree:**");
                        println!("    - **Path:** `{}`", worktree.path);
                        println!("    - **Branch:** `{}`", worktree.branch);

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
            println!("## Worktrees");
            println!();
            for worktree in orphan_worktrees {
                println!("### Worktree #{}", worktree.id);
                println!();
                println!("- **Path:** `{}`", worktree.path);
                println!("- **Branch:** `{}`", worktree.branch);

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

    fn handle_desc(&self, description: Option<&str>, task: Option<i64>) -> Result<()> {
        let task_id = match task {
            Some(id) => id,
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask)?,
        };

        let task_service = TaskService::new(&self.db);

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

    fn handle_ticket(&self, ticket_id: &str, url: &str, task: Option<i64>) -> Result<()> {
        let task_id = match task {
            Some(id) => id,
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask)?,
        };

        let task_service = TaskService::new(&self.db);
        task_service.link_ticket(task_id, ticket_id, url)?;

        println!("Linked ticket {} to task #{}", ticket_id, task_id);
        println!("URL: {}", url);

        Ok(())
    }

    fn handle_archive(&self, task_ref: Option<&str>) -> Result<()> {
        let task_service = TaskService::new(&self.db);
        let worktree_service = WorktreeService::new(&self.db);

        let task_id = match task_ref {
            Some(r) => task_service.resolve_task_id(r)?,
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask)?,
        };
        let task = task_service.get_task(task_id)?;

        // 1. Get all worktrees for this task
        let worktrees = worktree_service.list_worktrees(task_id)?;

        // 2. Check for uncommitted changes in worktrees that exist on disk
        let mut dirty_worktrees = Vec::new();
        for worktree in &worktrees {
            if std::path::Path::new(&worktree.path).exists()
                && worktree_service
                    .has_uncommitted_changes(&worktree.path)
                    .unwrap_or(false)
            {
                dirty_worktrees.push(worktree);
            }
        }

        if !dirty_worktrees.is_empty() {
            println!("WARNING: The following worktrees have uncommitted changes:");
            for wt in &dirty_worktrees {
                println!("  #{} {}", wt.id, wt.path);
            }
            println!();
            print!("Archive and remove worktrees anyway? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Cancelled.");
                return Ok(());
            }
        }

        // 3. Remove worktrees
        if !worktrees.is_empty() {
            println!("Cleaning up worktrees...");
            for worktree in worktrees {
                match worktree_service.remove_worktree(worktree.id, false) {
                    Ok(_) => {
                        println!("  Removed worktree #{}: {}", worktree.id, worktree.path);
                    }
                    Err(e) => {
                        eprintln!("  Error removing worktree #{}: {}", worktree.id, e);
                        // We continue even if one fails
                    }
                }
            }
        }

        task_service.archive_task(task_id)?;
        println!("Archived task #{}: {}", task.id, task.name);

        Ok(())
    }

    fn handle_todo(&self, command: TodoCommands) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;
        let todo_service = TodoService::new(&self.db);

        match command {
            TodoCommands::Add { text, worktree } => {
                let todo = todo_service.add_todo(current_task_id, &text, worktree)?;
                println!("Added TODO #{}: {}", todo.task_index, todo.content);

                if worktree {
                    println!("Worktree creation scheduled for 'track sync'");
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
                        Cell::new(&todo.status),
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
                // Resolve task_index to internal ID
                let todo = todo_service.get_todo_by_index(current_task_id, id)?;

                let worktree_service = WorktreeService::new(&self.db);
                if let Some(branch) = worktree_service.complete_worktree_for_todo(todo.id)? {
                    println!(
                        "Merged and removed worktree for TODO #{} (branch: {}).",
                        id, branch
                    );
                }

                todo_service.update_status(todo.id, "done")?;
                println!("Marked TODO #{} as done.", id);
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
        }

        Ok(())
    }

    fn handle_link(&self, command: LinkCommands) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;
        let link_service = LinkService::new(&self.db);

        match command {
            LinkCommands::Add { url, title } => {
                let link = link_service.add_link(current_task_id, &url, title.as_deref())?;
                println!("Added link #{}: {}", link.task_index, link.title);
            }
            LinkCommands::List => {
                let links = link_service.list_links(current_task_id)?;
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(Row::new(vec![
                    Cell::new("ID"),
                    Cell::new("Title"),
                    Cell::new("URL"),
                ]));

                for link in links {
                    table.add_row(Row::new(vec![
                        Cell::new(&link.task_index.to_string()),
                        Cell::new(&link.title),
                        Cell::new(&link.url),
                    ]));
                }

                table.printstd();
            }
            LinkCommands::Delete { index } => {
                let links = link_service.list_links(current_task_id)?;

                // Find link by task_index
                let link = links
                    .iter()
                    .find(|l| l.task_index == index as i64)
                    .ok_or_else(|| TrackError::Other(format!("Link #{} not found", index)))?;

                // Delete link via service
                link_service.delete_link(link.id)?;

                println!("Deleted link #{}: {}", index, link.title);
            }
        }

        Ok(())
    }

    fn handle_scrap(&self, command: ScrapCommands) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;
        let scrap_service = ScrapService::new(&self.db);

        match command {
            ScrapCommands::Add { content } => {
                let scrap = scrap_service.add_scrap(current_task_id, &content)?;
                let timestamp = scrap
                    .created_at
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S");
                println!("Added scrap at {}", timestamp);
            }
            ScrapCommands::List => {
                let scraps = scrap_service.list_scraps(current_task_id)?;
                for scrap in scraps {
                    let timestamp = scrap
                        .created_at
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    println!("[{}]", timestamp);
                    println!("  {}", scrap.content);
                    println!();
                }
            }
        }

        Ok(())
    }

    fn handle_sync(&self) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;

        let task_service = TaskService::new(&self.db);
        let task = task_service.get_task(current_task_id)?;
        let repo_service = RepoService::new(&self.db);
        let repos = repo_service.list_repos(current_task_id)?;

        if repos.is_empty() {
            return Err(TrackError::Other(
                "No repositories registered for this task".to_string(),
            ));
        }

        // Determine task branch name
        let task_branch = if let Some(ticket_id) = &task.ticket_id {
            format!("task/{}", ticket_id)
        } else {
            format!("task/task-{}", task.id)
        };

        println!("Syncing task branch: {}\n", task_branch);

        for repo in &repos {
            println!("Repository: {}", repo.repo_path);

            // Check if repository exists
            if !std::path::Path::new(&repo.repo_path).exists() {
                println!("  ⚠ Repository not found, skipping\n");
                continue;
            }

            // Check if branch exists
            let branch_check = std::process::Command::new("git")
                .args(["-C", &repo.repo_path, "rev-parse", "--verify", &task_branch])
                .output();

            let branch_exists = branch_check.map(|o| o.status.success()).unwrap_or(false);

            if !branch_exists {
                // Determine the base for creating the task branch
                let base_ref = if let Some(ref base_branch) = repo.base_branch {
                    // Use registered base branch
                    base_branch.clone()
                } else if let Some(ref base_hash) = repo.base_commit_hash {
                    // Use registered base commit hash
                    base_hash.clone()
                } else {
                    // Fallback: use current branch (for backward compatibility)
                    let current_branch_output = std::process::Command::new("git")
                        .args(["-C", &repo.repo_path, "rev-parse", "--abbrev-ref", "HEAD"])
                        .output()?;
                    String::from_utf8_lossy(&current_branch_output.stdout)
                        .trim()
                        .to_string()
                };

                // Create task branch from base
                let create_result = std::process::Command::new("git")
                    .args(["-C", &repo.repo_path, "branch", &task_branch, &base_ref])
                    .status();

                if create_result.is_ok() && create_result.unwrap().success() {
                    println!("  ✓ Branch {} created from {}", task_branch, base_ref);
                } else {
                    println!(
                        "  ✗ Failed to create branch {} from {}",
                        task_branch, base_ref
                    );
                    continue;
                }
            } else {
                println!("  ✓ Branch {} already exists", task_branch);
            }

            // Checkout task branch
            let checkout_result = std::process::Command::new("git")
                .args(["-C", &repo.repo_path, "checkout", &task_branch])
                .status();

            if checkout_result.is_ok() && checkout_result.unwrap().success() {
                println!("  ✓ Checked out {}\n", task_branch);
            } else {
                println!("  ✗ Failed to checkout {}\n", task_branch);
            }
        }

        // Check for pending worktrees
        println!("Checking for pending worktrees...");
        let todo_service = TodoService::new(&self.db);
        let worktree_service = WorktreeService::new(&self.db);
        let todos = todo_service.list_todos(current_task_id)?;

        for todo in todos {
            if todo.worktree_requested && todo.status != "done" {
                // Check if worktree already exists for this TODO
                let worktrees = worktree_service.list_worktrees(current_task_id)?;
                let mut exists = false;
                for wt in worktrees {
                    // Check if this worktree is linked to our todo
                    // Since list_worktrees returns GitItems which have todo_id, I need to check if that field is accessible
                    // Looking at models, GitItem has todo_id: Option<i64>
                    if wt.todo_id == Some(todo.id) {
                        exists = true;
                        break;
                    }
                }

                if !exists {
                    println!(
                        "Creating worktree for TODO #{}: {}",
                        todo.task_index, todo.content
                    );
                    for repo in &repos {
                        match worktree_service.add_worktree(
                            current_task_id,
                            &repo.repo_path,
                            None,
                            task.ticket_id.as_deref(),
                            Some(todo.id),
                            false,
                        ) {
                            Ok(wt) => println!("  Created {} ({})", wt.path, wt.branch),
                            Err(e) => {
                                eprintln!("  Error creating worktree for {}: {}", repo.repo_path, e)
                            }
                        }
                    }
                }
            }
        }

        println!("Sync complete.");
        Ok(())
    }

    fn handle_repo(&self, command: RepoCommands) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;
        let repo_service = RepoService::new(&self.db);

        match command {
            RepoCommands::Add { path, base } => {
                let repo_path = path.as_deref().unwrap_or(".");

                // Determine base branch and commit hash
                let (base_branch, base_commit_hash) = if let Some(branch) = base {
                    // User specified a base branch, get its commit hash
                    let hash_output = std::process::Command::new("git")
                        .args(["-C", repo_path, "rev-parse", &branch])
                        .output()?;

                    if !hash_output.status.success() {
                        return Err(TrackError::Other(format!(
                            "Failed to get commit hash for branch '{}'",
                            branch
                        )));
                    }

                    let hash = String::from_utf8_lossy(&hash_output.stdout)
                        .trim()
                        .to_string();
                    (Some(branch), Some(hash))
                } else {
                    // No base branch specified, use current branch
                    let branch_output = std::process::Command::new("git")
                        .args(["-C", repo_path, "rev-parse", "--abbrev-ref", "HEAD"])
                        .output()?;

                    if !branch_output.status.success() {
                        return Err(TrackError::Other(
                            "Failed to get current branch name".to_string(),
                        ));
                    }

                    let branch = String::from_utf8_lossy(&branch_output.stdout)
                        .trim()
                        .to_string();

                    // Get commit hash for current HEAD
                    let hash_output = std::process::Command::new("git")
                        .args(["-C", repo_path, "rev-parse", "HEAD"])
                        .output()?;

                    if !hash_output.status.success() {
                        return Err(TrackError::Other("Failed to get commit hash".to_string()));
                    }

                    let hash = String::from_utf8_lossy(&hash_output.stdout)
                        .trim()
                        .to_string();
                    (Some(branch), Some(hash))
                };

                let repo = repo_service.add_repo(
                    current_task_id,
                    repo_path,
                    base_branch.clone(),
                    base_commit_hash.clone(),
                )?;
                println!("Registered repository: {}", repo.repo_path);
                if let Some(branch) = base_branch {
                    println!(
                        "Base branch: {} ({})",
                        branch,
                        &base_commit_hash.unwrap()[..8]
                    );
                }
            }
            RepoCommands::List => {
                let repos = repo_service.list_repos(current_task_id)?;
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.set_titles(Row::new(vec![
                    Cell::new("ID"),
                    Cell::new("Repository Path"),
                ]));

                for repo in repos {
                    table.add_row(Row::new(vec![
                        Cell::new(&repo.task_index.to_string()),
                        Cell::new(&repo.repo_path),
                    ]));
                }

                table.printstd();
            }
            RepoCommands::Remove { id } => {
                let repos = repo_service.list_repos(current_task_id)?;

                // Find repo by task_index
                let repo = repos
                    .iter()
                    .find(|r| r.task_index == id)
                    .ok_or_else(|| TrackError::Other(format!("Repository #{} not found", id)))?;

                repo_service.remove_repo(repo.id)?;
                println!("Removed repository #{}", id);
            }
        }

        Ok(())
    }

    fn handle_alias(&self, command: AliasCommands) -> Result<()> {
        let task_service = TaskService::new(&self.db);

        match command {
            AliasCommands::Set { alias, task } => {
                let task_id = match task {
                    Some(id) => id,
                    None => self
                        .db
                        .get_current_task_id()?
                        .ok_or(TrackError::NoActiveTask)?,
                };

                task_service.set_alias(task_id, &alias)?;
                let task = task_service.get_task(task_id)?;
                println!("Set alias '{}' for task #{}: {}", alias, task.id, task.name);
            }
            AliasCommands::Remove { task } => {
                let task_id = match task {
                    Some(id) => id,
                    None => self
                        .db
                        .get_current_task_id()?
                        .ok_or(TrackError::NoActiveTask)?,
                };

                let task = task_service.get_task(task_id)?;
                if task.alias.is_none() {
                    println!("Task #{} has no alias", task_id);
                    return Ok(());
                }

                let removed_alias = task.alias.clone().unwrap();
                task_service.remove_alias(task_id)?;
                println!(
                    "Removed alias '{}' from task #{}: {}",
                    removed_alias, task.id, task.name
                );
            }
        }

        Ok(())
    }

    fn handle_completion(&self, shell: clap_complete::Shell, dynamic: bool) -> Result<()> {
        use clap::CommandFactory;
        use clap_complete::generate;
        use std::io;

        if dynamic {
            // Output dynamic completion script
            let script = match shell {
                clap_complete::Shell::Bash => {
                    include_str!("../../completions/track.bash.dynamic")
                }
                clap_complete::Shell::Zsh => {
                    include_str!("../../completions/_track.dynamic")
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

    fn handle_complete(&self, completion_type: CompletionType) -> Result<()> {
        match completion_type {
            CompletionType::Tasks => {
                // Output task IDs and names for 'track switch'
                let task_service = TaskService::new(&self.db);
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
                let current_task_id = self.db.get_current_task_id()?;
                if let Some(task_id) = current_task_id {
                    let todo_service = TodoService::new(&self.db);
                    let todos = todo_service.list_todos(task_id)?;

                    for todo in todos {
                        // Only show pending todos
                        if todo.status == "pending" {
                            // Format: ID:Content
                            println!("{}:{}", todo.task_index, todo.content);
                        }
                    }
                }
            }
            CompletionType::Links => {
                // Output link IDs and URLs for current task
                let current_task_id = self.db.get_current_task_id()?;
                if let Some(task_id) = current_task_id {
                    let link_service = LinkService::new(&self.db);
                    let links = link_service.list_links(task_id)?;

                    for link in links {
                        // Format: ID:Title:URL
                        println!("{}:{}:{}", link.task_index, link.title, link.url);
                    }
                }
            }
            CompletionType::Repos => {
                // Output repo IDs and paths for current task
                let current_task_id = self.db.get_current_task_id()?;
                if let Some(task_id) = current_task_id {
                    let repo_service = RepoService::new(&self.db);
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

    fn handle_llm_help(&self) -> Result<()> {
        println!(
            r#"# Track CLI Help for LLM Agents

## ⚠️ MANDATORY: Read This First

**BEFORE making ANY code changes, you MUST:**

1. Run `track sync` to create/checkout the task branch
2. Verify you are on the correct branch (NOT main/master/develop)
3. ONLY THEN begin coding

**FAILURE TO FOLLOW THIS WORKFLOW WILL RESULT IN:**
- Commits on the wrong branch (main/master)
- Merge conflicts that are difficult to resolve
- Loss of work isolation between tasks
- Broken git history

---

## LLM Agent Quick Start (MANDATORY STEPS)

When you start working on a task, follow these steps **IN ORDER**:

### Step 1: Sync (REQUIRED - DO THIS FIRST)
```bash
track sync
```
This creates the task branch and checks it out. **Do NOT skip this step.**

### Step 2: Verify Branch
```bash
git branch --show-current
```
Confirm the output shows `task/<ticket-id>` or `task/task-<id>`.
**If you see main/master/develop, STOP and run `track sync` again.**

### Step 3: Check Status
```bash
track status
```
Understand the current task, pending TODOs, and worktree paths.

### Step 4: Navigate to Worktree (if applicable)
If `track status` shows worktree paths, navigate to the appropriate one before making changes.

### Step 5: Execute Work
- Implement the required changes
- Run tests to verify
- Commit your changes with meaningful messages

### Step 6: Record Progress
```bash
track scrap add "Completed feature X, test Y passing"
```

### Step 7: Complete TODO
```bash
track todo done <index>
```

### Step 8: Repeat
Continue with the next pending TODO until all are complete.

---

## Overview

`track` is a CLI tool for managing development tasks, TODOs, and git worktrees.
This guide explains the standard workflow for completing tasks.

## Complete Task Workflow

### Phase 1: Task Setup (typically done by human)

1. **Create Task**: `track new "<task_name>"`
   - Creates a new task and switches to it.
   - Optionally link a ticket: `track new "<name>" --ticket PROJ-123 --ticket-url <url>`
   - Use template: `track new "<name>" --template <task_ref>` to copy TODOs from existing task

2. **Add Description**: `track desc "<description>"`
   - Provides detailed context about what needs to be done.

3. **Link Ticket** (if not done during creation): `track ticket <ticket_id> <url>`
   - Associates a Jira/GitHub/GitLab ticket with the task.
   - Enables ticket-based references and automatic branch naming.

4. **Register Repositories**: `track repo add [path]`
   - Register working repositories (default: current directory).
   - Optionally specify base branch: `track repo add --base <branch>`
   - Run this for each repository involved in the task.

5. **Add TODOs**: `track todo add "<content>" [--worktree]`
   - Add actionable items. Use `--worktree` flag to schedule worktree creation.

6. **Add Links**: `track link add <url> [--title "<title>"]`
   - Add reference links (documentation, PRs, issues, etc.)

### Phase 2: Task Execution (LLM or Human)

7. **Sync Repositories**: `track sync` **(MANDATORY FIRST STEP)**
   - Creates task branch on all registered repos.
   - Checks out the task branch.
   - Creates worktrees for TODOs that requested them.
   - **You MUST run this before making any code changes.**

8. **Verify Branch**: `git branch --show-current`
   - **STOP if output is main/master/develop. Run `track sync` again.**

9. **Check Current State**: `track status`
   - Shows current task, TODOs, worktrees, links, and recent scraps.
   - Use `track status --json` for structured output.
   - Use `track status --all` to show all scraps instead of recent ones.

10. **Execute TODOs**:
    - Navigate to worktree path if applicable (shown in `track status`).
    - Implement the required changes.
    - Run tests to verify.
    - Use `track scrap add "<note>"` to record findings, decisions, or progress.

11. **Complete TODO**: `track todo done <index>`
    - Marks the TODO as done.
    - If worktree exists: merges changes to task branch and removes worktree.

12. **Repeat** until all TODOs are complete.

## Key Commands Reference

| Command | Description |
|---------|-------------|
| `track sync` | **MANDATORY FIRST STEP** - Sync branches and create worktrees |
| `track status` | Show current task, TODOs, worktrees, links |
| `track status --json` | JSON output for programmatic access |
| `track status --all` | Show all scraps instead of recent |
| `track new "<name>"` | Create new task |
| `track new "<name>" --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track new "<name>" --template <ref>` | Create task from template (copies TODOs) |
| `track list` | List all tasks |
| `track desc [text]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link ticket to current task |
| `track switch <id>` | Switch to another task |
| `track switch t:<ticket_id>` | Switch by ticket reference |
| `track switch a:<alias>` | Switch by alias |
| `track archive [task_ref]` | Archive task (removes worktrees) |
| `track alias set <alias>` | Set alias for current task |
| `track alias remove` | Remove alias from current task |
| `track repo add [path]` | Register repository (default: current dir) |
| `track repo add --base <branch>` | Register with custom base branch |
| `track repo list` | List registered repositories |
| `track repo remove <index>` | Remove repository by task-scoped index |
| `track todo add "<text>"` | Add TODO |
| `track todo add "<text>" --worktree` | Add TODO with worktree |
| `track todo list` | List TODOs |
| `track todo done <index>` | Complete TODO (merges worktree if exists) |
| `track todo update <index> <status>` | Update TODO status |
| `track todo delete <index>` | Delete TODO |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with custom title |
| `track link list` | List all links |
| `track link delete <index>` | Delete link by task-scoped index |
| `track scrap add "<note>"` | Record work note |
| `track scrap list` | List all scraps |
| `track webui` | Start web-based UI (default: http://localhost:3000) |
| `track llm-help` | Show this help message |

## Task-Scoped Indices

**Important**: TODO, Link, and Repository indices are **task-scoped**, not global.
- Each task has its own numbering starting from 1
- When you switch tasks, indices reset to that task's scope
- This prevents confusion when working on multiple tasks

Example:
```bash
# Task 1 has TODOs: 1, 2, 3
track switch 1
track todo list  # Shows: 1, 2, 3

# Task 2 has TODOs: 1, 2 (different from Task 1's TODOs)
track switch 2
track todo list  # Shows: 1, 2
```

## Ticket Integration

### Linking Tickets
Tasks can be linked to external tickets (Jira, GitHub Issues, GitLab Issues):

**During task creation:**
```bash
track new "Fix login bug" --ticket PROJ-123 --ticket-url https://jira.example.com/browse/PROJ-123
```

**After task creation:**
```bash
track ticket PROJ-123 https://jira.example.com/browse/PROJ-123
```

### Ticket References
Once a ticket is linked, you can reference tasks by ticket ID:

```bash
# Switch to task by ticket ID
track switch t:PROJ-123

# Archive task by ticket ID
track archive t:PROJ-123

# View status by ticket ID
track status t:PROJ-123
```

### Automatic Branch Naming
When a ticket is linked, `track sync` automatically uses the ticket ID in branch names:

- **With ticket**: `task/PROJ-123` (and `task/PROJ-123-todo-1` for TODO worktrees)
- **Without ticket**: `task/task-42` (and `task/task-42-todo-1` for TODO worktrees)

This makes it easy to correlate branches with tickets in your issue tracker.

## Template Feature

Create new tasks based on existing ones:

```bash
# Create task from template (copies all TODOs)
track new "Sprint 2 Feature" --template t:PROJ-100

# TODOs are copied with status reset to 'pending'
# Useful for recurring workflows or similar tasks
```

## Web UI

Launch the web-based interface for visual task management:

```bash
track webui
```

Features:
- Real-time task status updates via Server-Sent Events (SSE)
- Visual TODO management with drag-and-drop
- Markdown rendering for scraps
- Inline editing of descriptions
- Link management
- Responsive design with dark mode

Access at: http://localhost:3000

## Important Notes

- **ALWAYS run `track sync` before making code changes.**
- **ALWAYS verify you are on the task branch, not main/master/develop.**
- TODO, Link, and Repository indices are **task-scoped**, not global.
- `track todo done` automatically merges and removes associated worktrees.
- Always register repos with `track repo add` before running `track sync`.
- Use `track scrap add` to document decisions and findings during work.
- Ticket IDs are used in branch names when linked (e.g., `task/PROJ-123`).
- Scraps support Markdown formatting and are rendered as HTML in WebUI.

## Detailed Specifications

### Worktree Location
Worktrees are created as subdirectories inside the registered repository:
- **Path**: `<repo_root>/<branch_name>`
- **Example**: `/src/my-app/task/PROJ-123-todo-1`

### TODO Completion Process
Executing `track todo done <index>` performs the following:
1. **Checks** for uncommitted changes in the TODO worktree (must be clean).
2. **Merges** the TODO branch into the Task Base branch (in the base worktree).
3. **Removes** the TODO worktree directory and DB record.
4. **Updates** TODO status to 'done'.

### Archive Process
Executing `track archive` performs the following:
1. **Checks** for uncommitted changes in all worktrees.
2. **Prompts** for confirmation if dirty worktrees are found.
3. **Removes** all worktrees associated with the task.
4. **Marks** the task as archived.
"#
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_llm_help() {
        let db = Database::new_in_memory().unwrap();
        let handler = CommandHandler::from_db(db);

        let result = handler.handle_llm_help();
        assert!(result.is_ok());
    }

    #[test]
    fn test_completion() {
        let db = Database::new_in_memory().unwrap();
        let handler = CommandHandler::from_db(db);

        // Test that completion handler runs without error for each shell
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
        ] {
            let result = handler.handle_completion(shell);
            assert!(
                result.is_ok(),
                "Completion generation failed for {:?}",
                shell
            );
        }
    }
}
