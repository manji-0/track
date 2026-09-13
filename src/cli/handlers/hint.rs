//! Post-state / next-action hints appended to CLI output (stderr).

use crate::cli::handlers::CommandCtx;
use crate::models::TaskId;
use crate::services::WorktreeService;
use crate::services::agent_context::build_agent_extensions;
use crate::use_cases::GetTaskInfoUseCase;
use crate::utils::Result;

/// Print a compact hint footer unless `--json` or `TRACK_HINTS=0`.
pub fn emit_hint(ctx: &CommandCtx, json: bool, task_id: Option<TaskId>) -> Result<()> {
    if json || hints_disabled() {
        return Ok(());
    }

    let Some(task_id) = task_id.or(ctx.db.get_current_task_id()?) else {
        return Ok(());
    };

    let snapshot = GetTaskInfoUseCase::new(ctx.db).load(task_id)?;
    let worktree_service = WorktreeService::new(ctx.db);
    let extensions = build_agent_extensions(
        snapshot.vcs_mode,
        snapshot.aggressive_mode,
        &snapshot.task,
        &snapshot.todos,
        &snapshot.worktrees,
        &snapshot.repos,
        &worktree_service,
    );

    eprintln!();
    for line in extensions.hint.stderr_lines() {
        eprintln!("{line}");
    }
    Ok(())
}

fn hints_disabled() -> bool {
    matches!(
        std::env::var("TRACK_HINTS").ok().as_deref(),
        Some("0") | Some("off") | Some("false")
    )
}
