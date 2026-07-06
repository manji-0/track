use crate::cli::handlers::CommandCtx;
use crate::cli::InstallAgentsArgs;
use crate::services::agents_md::{
    install_agents_section, resolve_agents_path, AgentsInstallAction, AgentsInstallScope,
};
use crate::utils::{Result, TrackError};
use std::path::Path;
use std::process::Command;

pub fn handle_install_agents(_ctx: &CommandCtx, args: &InstallAgentsArgs) -> Result<()> {
    let scope = resolve_scope(args)?;
    let path = resolve_agents_path(scope, args.path.as_deref())?;

    let report = install_agents_section(&path, args.dry_run, scope)?;

    match report.action {
        AgentsInstallAction::Created => {
            if args.dry_run {
                println!("Would create {}", display_path(&report.path));
            } else {
                println!("Created {}", display_path(&report.path));
            }
        }
        AgentsInstallAction::Updated => {
            if args.dry_run {
                println!("Would update {}", display_path(&report.path));
            } else {
                println!("Updated {}", display_path(&report.path));
            }
        }
        AgentsInstallAction::Unchanged => {
            println!("Already up to date: {}", display_path(&report.path));
        }
    }

    if args.skills && !args.dry_run {
        install_skills()?;
    } else if args.skills {
        println!("Skipping skill install in --dry-run mode");
        println!("Would run: npx skills add manji-0/track -s track -s track-task-execute -g -y");
    }

    if args.dry_run {
        println!("\nManaged section preview:\n");
        println!("{}", crate::services::agents_md::managed_section());
    }

    Ok(())
}

fn resolve_scope(args: &InstallAgentsArgs) -> Result<AgentsInstallScope> {
    let selected = [args.global, args.project, args.path.is_some()]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if selected > 1 {
        return Err(TrackError::Other(
            "use only one of --global, --project, or --path".into(),
        ));
    }

    if args.path.is_some() {
        Ok(AgentsInstallScope::Custom)
    } else if args.project {
        Ok(AgentsInstallScope::Project)
    } else {
        Ok(AgentsInstallScope::Global)
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn install_skills() -> Result<()> {
    println!("\nInstalling track skills (global)...");
    let track_status = Command::new("npx")
        .args([
            "skills",
            "add",
            "manji-0/track",
            "-s",
            "track",
            "-s",
            "track-task-setup",
            "-s",
            "track-task-execute",
            "-s",
            "track-advanced",
            "-g",
            "-a",
            "cursor",
            "-a",
            "claude-code",
            "-a",
            "codex",
            "-y",
        ])
        .status()
        .map_err(|err| {
            TrackError::Other(format!("failed to run npx (is Node.js installed?): {err}"))
        })?;
    if !track_status.success() {
        return Err(TrackError::Other(
            "track skill install failed (see npx output above)".into(),
        ));
    }

    println!("Installing agent-skill-jj ($jj)...");
    let jj_status = Command::new("npx")
        .args([
            "skills",
            "add",
            "manji-0/agent-skill-jj",
            "-s",
            "jj",
            "-g",
            "-y",
        ])
        .status()
        .map_err(|err| TrackError::Other(format!("failed to run npx: {err}")))?;
    if !jj_status.success() {
        return Err(TrackError::Other(
            "agent-skill-jj skill install failed (see npx output above)".into(),
        ));
    }

    println!("Skills installed. Verify with: npx skills list | rg track");
    Ok(())
}
