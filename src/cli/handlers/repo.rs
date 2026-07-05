use crate::cli::handlers::CommandCtx;
use crate::cli::RepoCommands;
use crate::services::RepoService;
use crate::utils::{Result, TrackError};
use prettytable::{format, Cell, Row, Table};

pub fn handle_repo(ctx: &CommandCtx, command: RepoCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let repo_service = RepoService::new(ctx.db);

    match command {
        RepoCommands::Add { path, base } => {
            let repo_path = path.as_deref().unwrap_or(".");

            // Determine base bookmark and change ID
            let (base_branch, base_commit_hash) = if let Some(bookmark) = base {
                let hash_output = std::process::Command::new("jj")
                    .args([
                        "-R",
                        repo_path,
                        "log",
                        "-r",
                        &bookmark,
                        "--no-graph",
                        "-T",
                        "commit_id",
                    ])
                    .output()?;

                if !hash_output.status.success() {
                    return Err(TrackError::Other(format!(
                        "Failed to get change ID for bookmark '{}'",
                        bookmark
                    )));
                }

                let hash = String::from_utf8_lossy(&hash_output.stdout)
                    .trim()
                    .to_string();
                (Some(bookmark), Some(hash))
            } else {
                let bookmark_output = std::process::Command::new("jj")
                    .args(["-R", repo_path, "bookmark", "list", "-r", "@", "-T", "name"])
                    .output()?;

                if !bookmark_output.status.success() {
                    return Err(TrackError::Other(
                        "Failed to resolve current bookmark".to_string(),
                    ));
                }

                let bookmark = String::from_utf8_lossy(&bookmark_output.stdout)
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string());

                let hash = if let Some(ref name) = bookmark {
                    let hash_output = std::process::Command::new("jj")
                        .args([
                            "-R",
                            repo_path,
                            "log",
                            "-r",
                            name,
                            "--no-graph",
                            "-T",
                            "commit_id",
                        ])
                        .output()?;

                    if !hash_output.status.success() {
                        return Err(TrackError::Other(
                            "Failed to get change ID for current bookmark".to_string(),
                        ));
                    }

                    Some(
                        String::from_utf8_lossy(&hash_output.stdout)
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                };

                (bookmark, hash)
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
                    "Base bookmark: {} ({})",
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
