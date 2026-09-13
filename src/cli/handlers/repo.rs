use crate::cli::RepoCommands;
use crate::cli::handlers::CommandCtx;
use crate::cli::handlers::hint::emit_hint;
use crate::cli::handlers::json_out::{MutationKind, emit_mutation};
use crate::services::{RepoService, task_workspace};
use crate::use_cases::SyncTaskUseCase;
use crate::utils::{Result, TrackError};
use prettytable::{Cell, Row, Table, format};

pub fn handle_repo(ctx: &CommandCtx, command: RepoCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let repo_service = RepoService::new(ctx.db);

    match command {
        RepoCommands::Add { path, base, json } => {
            let repo_path = path.as_deref().unwrap_or(".");
            let vcs_mode = ctx.db.get_vcs_mode()?;
            let (base_branch, base_commit_hash) =
                task_workspace::resolve_base(vcs_mode, repo_path, base.as_deref())?;

            let repo = repo_service.add_repo(
                current_task_id,
                repo_path,
                base_branch.clone(),
                base_commit_hash.clone(),
            )?;

            let sync_note = match SyncTaskUseCase::new(ctx.db).execute(current_task_id, false) {
                Ok(_) => None,
                Err(err) => Some(err.to_string()),
            };

            emit_mutation(
                ctx,
                json,
                MutationKind::RepoAdd,
                Some(repo.task_index.as_i64()),
                Some(current_task_id),
                || {
                    println!("Registered repository: {}", repo.repo_path);
                    if let Some(branch) = &base_branch {
                        if let Some(hash) = &base_commit_hash {
                            let short = &hash[..std::cmp::min(8, hash.len())];
                            println!("Base: {branch} ({short})");
                        } else {
                            println!("Base: {branch}");
                        }
                    }
                    if let Some(err) = &sync_note {
                        eprintln!("Workspace not created yet: {err}");
                        eprintln!("Run `track sync` after the base workspace is clean.");
                    }
                },
            )?;
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
            emit_hint(ctx, false, Some(current_task_id))?;
        }
        RepoCommands::Remove { id } => {
            let repos = repo_service.list_repos(current_task_id)?;

            let repo = repos
                .iter()
                .find(|r| r.task_index == id)
                .ok_or(TrackError::TaskRepoIndexNotFound(id))?;

            repo_service.remove_repo(repo.id)?;
            println!("Removed repository #{}", id);
            emit_hint(ctx, false, Some(current_task_id))?;
        }
    }

    Ok(())
}
