use track::cli::handler::CommandHandler;
use track::cli::{Commands, LinkCommands, ScrapCommands, TodoCommands};
use track::db::Database;
use track::services::{LinkService, ScrapService, TaskService, TodoService};

#[test]
fn test_handle_list_no_output_errors() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    // Create mix of active and archived tasks
    let t1 = task_service
        .create_task("Active 1", None, None, None)
        .unwrap();
    let t2 = task_service
        .create_task("Active 2", None, None, None)
        .unwrap();
    task_service.archive_task(t1.id).unwrap();

    // List active only - should not error
    let cmd = Commands::List { all: false };
    assert!(handler.handle(cmd).is_ok());

    // List all - should not error
    let cmd = Commands::List { all: true };
    assert!(handler.handle(cmd).is_ok());
}

#[test]
fn test_handle_info_no_output_errors() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let todo_service = TodoService::new(db);
    let link_service = LinkService::new(db);
    let scrap_service = ScrapService::new(db);

    let task = task_service
        .create_task(
            "Task",
            Some("Description"),
            Some("INFO-1"),
            Some("http://url"),
        )
        .unwrap();

    // Add various items
    let _t1 = todo_service.add_todo(task.id, "Pending", false).unwrap();
    let t2 = todo_service.add_todo(task.id, "Done", false).unwrap();
    todo_service.update_status(t2.id, "done").unwrap();
    let t3 = todo_service.add_todo(task.id, "Cancelled", false).unwrap();
    todo_service.update_status(t3.id, "cancelled").unwrap();

    link_service
        .add_link(task.id, "http://example.com", Some("Example"))
        .unwrap();
    scrap_service.add_scrap(task.id, "Scrap note").unwrap();

    // Info without JSON - should not error
    let cmd = Commands::Status { json: false };
    assert!(handler.handle(cmd).is_ok());

    // Info with JSON - should not error
    let cmd = Commands::Status { json: true };
    assert!(handler.handle(cmd).is_ok());
}

#[test]
fn test_handle_desc_view_and_set() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    // View mode (no description) - should not error
    let cmd = Commands::Desc {
        description: None,
        task: None,
    };
    assert!(handler.handle(cmd).is_ok());

    // Set mode - should not error
    let cmd = Commands::Desc {
        description: Some("New description".to_string()),
        task: None,
    };
    assert!(handler.handle(cmd).is_ok());

    // Verify it was set
    let updated_task = task_service.get_task(task.id).unwrap();
    assert_eq!(updated_task.description.as_deref(), Some("New description"));
}

#[test]
fn test_handle_ticket_links() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    // Link ticket - should not error
    let cmd = Commands::Ticket {
        ticket_id: "TICK-123".to_string(),
        url: "http://ticket.com".to_string(),
        task: None,
    };
    assert!(handler.handle(cmd).is_ok());

    // Verify linked
    let updated_task = task_service.get_task(task.id).unwrap();
    assert_eq!(updated_task.ticket_id.as_deref(), Some("TICK-123"));
    assert_eq!(
        updated_task.ticket_url.as_deref(),
        Some("http://ticket.com")
    );
}

#[test]
fn test_handle_llm_help_no_errors() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);

    // Should just print help text, no errors
    let cmd = Commands::LlmHelp;
    assert!(handler.handle(cmd).is_ok());
}

#[test]
fn test_handle_todo_list_empty() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let _task = task_service.create_task("Task", None, None, None).unwrap();

    // List empty todos - should not error
    let cmd = Commands::Todo(TodoCommands::List);
    assert!(handler.handle(cmd).is_ok());
}

#[test]
fn test_handle_link_list_empty() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let _task = task_service.create_task("Task", None, None, None).unwrap();

    // List empty links - should not error
    let cmd = Commands::Link(LinkCommands::List);
    assert!(handler.handle(cmd).is_ok());
}

#[test]
fn test_handle_scrap_list_empty() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let _task = task_service.create_task("Task", None, None, None).unwrap();

    // List empty scraps - should not error
    let cmd = Commands::Scrap(ScrapCommands::List);
    assert!(handler.handle(cmd).is_ok());
}
