use crate::cli::handlers::CommandCtx;
use crate::cli::LinkCommands;
use crate::services::LinkService;
use crate::utils::{Result, TrackError};
use prettytable::{format, Cell, Row, Table};

pub fn handle_link(ctx: &CommandCtx, command: LinkCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let link_service = LinkService::new(ctx.db);

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
                .ok_or(TrackError::LinkIndexNotFound(index as i64))?;

            // Delete link via service
            link_service.delete_link(link.id)?;

            println!("Deleted link #{}: {}", index, link.title);
        }
    }

    Ok(())
}
