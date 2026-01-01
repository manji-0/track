use crate::cli::{Commands, LinkCommands, RepoCommands, ScrapCommands, TodoCommands};
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

    pub fn handle(&self, command: Commands) -> Result<()> {
        match command {
            Commands::New {
                name,
                description,
                ticket,
                ticket_url,
            } => self.handle_new(
                &name,
                description.as_deref(),
                ticket.as_deref(),
                ticket_url.as_deref(),
            ),
            Commands::List { all } => self.handle_list(all),
            Commands::Switch { task_ref } => self.handle_switch(&task_ref),
            Commands::Info { json } => self.handle_info(json),
            Commands::Desc { description, task } => self.handle_desc(description.as_deref(), task),
            Commands::Ticket {
                ticket_id,
                url,
                task,
            } => self.handle_ticket(&ticket_id, &url, task),
            Commands::Archive { task_ref } => self.handle_archive(&task_ref),
            Commands::Todo(cmd) => self.handle_todo(cmd),
            Commands::Link(cmd) => self.handle_link(cmd),
            Commands::Scrap(cmd) => self.handle_scrap(cmd),
            Commands::Sync => self.handle_sync(),
            Commands::Repo(cmd) => self.handle_repo(cmd),
            Commands::LlmHelp => self.handle_llm_help(),
        }
    }

    fn handle_new(
        &self,
        name: &str,
        description: Option<&str>,
        ticket: Option<&str>,
        ticket_url: Option<&str>,
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

    fn handle_info(&self, json: bool) -> Result<()> {
        let current_task_id = self
            .db
            .get_current_task_id()?
            .ok_or(TrackError::NoActiveTask)?;

        let task_service = TaskService::new(&self.db);
        let task = task_service.get_task(current_task_id)?;

        let todo_service = TodoService::new(&self.db);
        let todos = todo_service.list_todos(current_task_id)?;

        let link_service = LinkService::new(&self.db);
        let links = link_service.list_links(current_task_id)?;

        let scrap_service = ScrapService::new(&self.db);
        let scraps = scrap_service.list_scraps(current_task_id)?;

        let worktree_service = WorktreeService::new(&self.db);
        let worktrees = worktree_service.list_worktrees(current_task_id)?;

        if json {
            let mut worktrees_json = Vec::new();
            for wt in worktrees {
                let repo_links = worktree_service.list_repo_links(wt.id)?;
                let mut wt_val = serde_json::to_value(&wt).unwrap_or(serde_json::Value::Null);
                if let Some(obj) = wt_val.as_object_mut() {
                    obj.insert(
                        "repo_links".to_string(),
                        serde_json::to_value(&repo_links).unwrap_or(serde_json::Value::Null),
                    );
                }
                worktrees_json.push(wt_val);
            }

            let output = serde_json::json!({
                "task": task,
                "todos": todos,
                "links": links,
                "scraps": scraps,
                "worktrees": worktrees_json,
            });

            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return Ok(());
        }

        println!("=== Task #{}: {} ===", task.id, task.name);
        if let Some(ticket_id) = &task.ticket_id {
            print!("Ticket: {}", ticket_id);
            if let Some(url) = &task.ticket_url {
                print!(" ({})", url);
            }
            println!();
        }
        let created = task
            .created_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S");
        println!("Created: {}", created);
        println!();

        // Description
        if let Some(desc) = &task.description {
            println!("Description:");
            println!("  {}", desc);
            println!();
        }

        // TODOs
        if !todos.is_empty() {
            println!("[ TODOs ]");
            for todo in todos {
                let marker = match todo.status.as_str() {
                    "done" => "[x]",
                    "cancelled" => "[-]",
                    _ => "[ ]",
                };
                println!("  {} [{}] {}", marker, todo.task_index, todo.content);
            }
            println!();
        }

        // Links
        if !links.is_empty() {
            println!("[ Links ]");
            for link in links {
                println!("  - {}: {}", link.title, link.url);
            }
            println!();
        }

        // Scraps
        if !scraps.is_empty() {
            println!("[ Recent Scraps ]");
            for scrap in scraps.iter().rev().take(5) {
                let timestamp = scrap.created_at.with_timezone(&Local).format("%H:%M");
                println!("  [{}] {}", timestamp, scrap.content);
            }
            println!();
        }

        // Worktrees
        if !worktrees.is_empty() {
            println!("[ Worktrees ]");
            for worktree in worktrees {
                println!("  #{} {} ({})", worktree.id, worktree.path, worktree.branch);
                let repo_links = worktree_service.list_repo_links(worktree.id)?;
                for link in repo_links {
                    println!("      └─ {}: {}", link.kind, link.url);
                }
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

    fn handle_archive(&self, task_ref: &str) -> Result<()> {
        let task_service = TaskService::new(&self.db);
        let worktree_service = WorktreeService::new(&self.db);

        let task_id = task_service.resolve_task_id(task_ref)?;
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
                println!("Added link #{}: {}", link.id, link.title);
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
                        Cell::new(&link.id.to_string()),
                        Cell::new(&link.title),
                        Cell::new(&link.url),
                    ]));
                }

                table.printstd();
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
                // Get current branch
                let current_branch_output = std::process::Command::new("git")
                    .args(["-C", &repo.repo_path, "rev-parse", "--abbrev-ref", "HEAD"])
                    .output()?;
                let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
                    .trim()
                    .to_string();

                // Create task branch
                let create_result = std::process::Command::new("git")
                    .args(["-C", &repo.repo_path, "branch", &task_branch])
                    .status();

                if create_result.is_ok() && create_result.unwrap().success() {
                    println!("  ✓ Branch {} created from {}", task_branch, current_branch);
                } else {
                    println!("  ✗ Failed to create branch {}", task_branch);
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
            RepoCommands::Add { path } => {
                let repo_path = path.as_deref().unwrap_or(".");
                let repo = repo_service.add_repo(current_task_id, repo_path)?;
                println!("Registered repository: {}", repo.repo_path);
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
                        Cell::new(&repo.id.to_string()),
                        Cell::new(&repo.repo_path),
                    ]));
                }

                table.printstd();
            }
            RepoCommands::Remove { id } => {
                repo_service.remove_repo(id)?;
                println!("Removed repository #{}", id);
            }
        }

        Ok(())
    }

    fn handle_llm_help(&self) -> Result<()> {
        println!(
            r#"# WorkTracker CLI Help for LLM Agents

## Overview

`track` is a CLI tool for managing development tasks, TODOs, and git worktrees.
This guide explains the standard workflow for completing tasks.

## Complete Task Workflow

### Phase 1: Task Setup (typically done by human)

1. **Create Task**: `track new "<task_name>"`
   - Creates a new task and switches to it.

2. **Add Description**: `track desc "<description>"`
   - Provides detailed context about what needs to be done.

3. **Register Repositories**: `track repo add [path]`
   - Register working repositories (default: current directory).
   - Run this for each repository involved in the task.

4. **Add TODOs**: `track todo add "<content>" [--worktree]`
   - Add actionable items. Use `--worktree` flag to schedule worktree creation.

### Phase 2: Task Execution (LLM or Human)

5. **Sync Repositories**: `track sync`
   - Creates task branch on all registered repos.
   - Creates worktrees for TODOs that requested them.
   - Run this from any registered repository.

6. **Check Current State**: `track info`
   - Shows current task, TODOs, worktrees, and recent scraps.
   - Use `track info --json` for structured output.

7. **Execute TODOs**:
   - Navigate to worktree path if applicable (shown in `track info`).
   - Implement the required changes.
   - Run tests to verify.
   - Use `track scrap add "<note>"` to record findings, decisions, or progress.

8. **Complete TODO**: `track todo done <index>`
   - Marks the TODO as done.
   - If worktree exists: merges changes to task branch and removes worktree.

9. **Repeat** until all TODOs are complete.

## Key Commands Reference

| Command | Description |
|---------|-------------|
| `track info` | Show current task, TODOs, worktrees |
| `track info --json` | JSON output for programmatic access |
| `track new "<name>"` | Create new task |
| `track desc [text]` | View or set task description |
| `track switch <id>` | Switch to another task |
| `track repo add [path]` | Register repository |
| `track repo list` | List registered repositories |
| `track todo add "<text>"` | Add TODO |
| `track todo add "<text>" --worktree` | Add TODO with worktree |
| `track todo list` | List TODOs |
| `track todo done <index>` | Complete TODO |
| `track sync` | Sync branches and create worktrees |
| `track scrap add "<note>"` | Record work note |
| `track scrap list` | List all scraps |

## LLM Agent Quick Start

When you start working on a task:

1. Run `track info` to understand the current state.
2. Identify the next pending TODO.
3. If worktree paths are shown, navigate to the appropriate one.
4. Implement changes and run tests.
5. Record progress with `track scrap add`.
6. Complete with `track todo done <index>`.

## Important Notes

- TODO indices (1, 2, 3...) are **task-scoped**, not global.
- `track todo done` automatically merges and removes associated worktrees.
- Always register repos with `track repo add` before running `track sync`.
- Use `track scrap add` to document decisions and findings during work.

## Detailed Specifications

### Worktree Location
Worktrees are created as subdirectories inside the registered repository:
- **Path**: `<repo_root>/<branch_name>`
- **Example**: `/src/my-app/PROJ-123-todo-1`

### TODO Completion Process
Executing `track todo done <index>` performs the following:
1. **Checks** for uncommitted changes in the TODO worktree (must be clean).
2. **Merges** the TODO branch into the Task Base branch (in the base worktree).
3. **Removes** the TODO worktree directory and DB record.
4. **Updates** TODO status to 'done'.
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
}
