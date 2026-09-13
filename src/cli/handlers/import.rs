use crate::cli::handlers::CommandCtx;
use crate::cli::handlers::json_out::{MutationKind, emit_mutation};
use crate::use_cases::ImportTaskNotesUseCase;
use crate::utils::{Result, TrackError};
use std::env;
use std::path::PathBuf;

pub fn handle_import(ctx: &CommandCtx, path: Option<&str>, json: bool) -> Result<()> {
    let repo = match path {
        Some(p) => PathBuf::from(p),
        None => env::current_dir()?,
    };
    if !repo.exists() {
        return Err(TrackError::PathResolutionFailed(repo.display().to_string()));
    }

    let outcome = ImportTaskNotesUseCase::new(ctx.db).execute(&repo)?;
    emit_mutation(
        ctx,
        json,
        MutationKind::Import,
        Some(outcome.task.id.as_i64()),
        Some(outcome.task.id),
        || {
            println!(
                "Imported task #{}: {} (slug {})",
                outcome.task.id, outcome.task.name, outcome.slug
            );
            println!(
                "  {} TODOs, {} shared scraps, {} links",
                outcome.todos, outcome.scraps, outcome.links
            );
            if outcome.alias_set {
                println!("  alias set to {}", outcome.slug);
            }
            if let Some(skip) = &outcome.ticket_skipped {
                println!("  ticket not applied: {skip}");
            }
            if outcome.repo_registered {
                println!("  registered repo {}", repo.display());
                println!("  next: track sync");
            } else {
                println!("  next: track repo add . && track sync");
            }
        },
    )
}
