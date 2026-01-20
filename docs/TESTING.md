# Test Documentation

This document describes the test configuration and strategy for the `track` CLI project.

## Test Overview

### Test Statistics
- **Total Tests**: 79
  - Unit Tests: 74
  - Integration Tests: 5

### Test Coverage

#### 1. Models (`src/models/mod.rs`)
**Number of Tests**: 4

- `test_task_status_as_str`: Test for TaskStatus `as_str` method
- `test_task_status_from_str`: Test for TaskStatus `from_str` method
- `test_todo_status_as_str`: Test for TodoStatus `as_str` method
- `test_todo_status_from_str`: Test for TodoStatus `from_str` method

**Coverage**: ✅ Complete
- Tests conversion for all enum variants
- Tests handling of invalid inputs

#### 2. Database (`src/db/mod.rs`)
**Number of Tests**: 4

- `test_new_in_memory`: Test creation of in-memory DB
- `test_app_state_get_set`: Test getting/setting application state
- `test_current_task_id`: Test current task ID management
- `test_schema_initialization`: Test schema initialization

**Coverage**: ✅ Complete
- DB connection and schema initialization
- State management functionality
- All public methods

#### 3. TaskService (`src/services/task_service.rs`)
**Number of Tests**: 18

**Basic CRUD Operations**:
- `test_create_task_success`: Success case for task creation
- `test_create_task_with_ticket`: Task creation with ticket
- `test_create_task_with_description`: Task creation with description
- `test_create_task_empty_name`: Error handling for empty name
- `test_create_task_duplicate_ticket`: Error handling for duplicate ticket
- `test_get_task_success`: Success case for task retrieval
- `test_get_task_not_found`: Error handling for non-existent task
- `test_list_tasks`: Task list retrieval
- `test_list_tasks_exclude_archived`: Exclusion of archived tasks

**Task Management**:
- `test_switch_task_success`: Success case for task switching
- `test_switch_task_archived`: Error switching to archived task
- `test_archive_task`: Task archiving

**Ticket Management**:
- `test_link_ticket_success`: Success case for linking ticket
- `test_link_ticket_duplicate`: Error handling for duplicate ticket
- `test_validate_ticket_format_jira`: Validation of JIRA ticket format
- `test_validate_ticket_format_github`: Validation of GitHub ticket format
- `test_validate_ticket_format_invalid`: Error handling for invalid ticket format

**Description Management**:
- `test_set_description`: Setting description
- `test_set_description_archived_task`: Error setting description for archived task
- `test_description_persists`: Verifying description matches persistence

**Task Resolution**:
- `test_resolve_task_id_by_id`: Task resolution by ID
- `test_resolve_task_id_by_ticket`: Task resolution by ticket ID

**Coverage**: ✅ Complete
- All public methods
- Success and error cases
- Edge cases

#### 4. TodoService (`src/services/todo_service.rs`)
**Number of Tests**: 15

**Basic CRUD Operations**:
- `test_add_todo_success`: Success case for adding TODO
- `test_add_todo_with_worktree_success`: Adding TODO with workspace
- `test_get_todo_success`: Success case for TODO retrieval
- `test_get_todo_not_found`: Error handling for non-existent TODO
- `test_list_todos`: TODO list retrieval
- `test_delete_todo_success`: Success case for TODO deletion
- `test_delete_todo_not_found`: Error handling for deleting non-existent TODO

**Status Management**:
- `test_update_status_success`: Success case for status update
- `test_update_status_invalid`: Error handling for invalid status
- `test_update_status_not_found`: Error updating status for non-existent TODO

**Task Index Management**:
- `test_task_index_sequential`: Sequentiality of task index
- `test_task_index_independence`: Independence of indices between tasks
- `test_get_todo_by_index_success`: Success case for TODO retrieval by index
- `test_get_todo_by_index_not_found`: Error handling for non-existent index
- `test_list_todos_ordered_by_index`: TODO list ordered by index

**Coverage**: ✅ Complete
- All public methods
- Task-scoped index management
- Error handling

#### 5. LinkService & ScrapService (`src/services/link_service.rs`)
**Number of Tests**: 10

**LinkService (7 Tests)**:
- `test_add_link_success`: Success case for adding link
- `test_add_link_default_title`: Adding link with default title
- `test_add_link_invalid_url`: Error handling for invalid URL
- `test_list_links`: Link list retrieval
- `test_validate_url_http`: Validation of HTTP URL
- `test_validate_url_https`: Validation of HTTPS URL
- `test_validate_url_invalid`: Validation error for invalid URL

**ScrapService (3 Tests)**:
- `test_add_scrap_success`: Success case for adding scrap
- `test_get_scrap_success`: Success case for scrap retrieval
- `test_list_scraps`: Scrap list retrieval (chronological order)

**Coverage**: ✅ Complete
- All public methods
- URL validation logic
- Chronological sorting

#### 6. RepoService (`src/services/repo_service.rs`)
**Number of Tests**: 5

- `test_add_repo_success`: Success case for repository registration
- `test_add_repo_not_git`: Error handling for non-JJ directory
- `test_add_repo_duplicate`: Error handling for duplicate repository
- `test_list_repos`: Repository list retrieval
- `test_remove_repo`: Repository removal

**Coverage**: ✅ Complete
- All public methods
- JJ validation logic
- Duplicate checks

#### 7. WorkspaceService (`src/services/worktree_service.rs`)
**Number of Tests**: 13

**Bookmark Name Determination Logic (6 Tests)**:
- `test_determine_branch_name_with_explicit_branch_and_ticket`
- `test_determine_branch_name_with_explicit_branch_only`
- `test_determine_branch_name_with_ticket_and_todo`
- `test_determine_branch_name_with_todo_only`
- `test_determine_branch_name_base_with_ticket`
- `test_determine_branch_name_base_without_ticket`

**Workspace Operations (6 Tests)**:
- `test_add_worktree_and_get`: Workspace addition and retrieval
- `test_list_worktrees`: Workspace list retrieval
- `test_remove_worktree`: Workspace removal
- `test_get_base_worktree`: Base workspace retrieval
- `test_get_worktree_by_todo`: Retrieval of workspace by TODO
- `test_determine_worktree_path`: Workspace path determination

**JJ Operations (1 Test)**:
- `test_has_uncommitted_changes`: Detection of uncommitted changes

**Coverage**: ✅ Complete
- All public methods
- All patterns of bookmark naming strategy
- Integration tests using actual JJ repositories

#### 8. CommandHandler (`src/cli/handler.rs`)
**Number of Tests**: 1

- `test_llm_help`: Execution test for LLM help command

**Coverage**: ⚠️ Partial
- Only LLM help command is tested
- Other CLI commands are covered by integration tests

### Integration Tests (`tests/integration_test.rs`)
**Number of Tests**: 5

1. **test_full_task_workflow**
   - Complete workflow from task creation to completion
   - TODO addition, status update, archiving

2. **test_repo_worktree_workflow**
   - Repository registration
   - Base workspace creation
   - Workspace list retrieval
   - Repository removal

3. **test_task_switching**
   - Switching between multiple tasks
   - Current task management
   - Exclusion of archived tasks

4. **test_todo_task_index_independence**
   - Independence of TODO indices across multiple tasks
   - Task-scoped index management

5. **test_error_handling**
   - Access errors for non-existent resources
   - Operation errors for archived tasks
   - Error handling for invalid operations

## How to Run Tests

### Run All Tests
```bash
cargo test --all
```

### Run Specific Tests
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_test

# Tests for a specific module
cargo test services::task_service

# Specific test function
cargo test test_create_task_success
```

### Detailed Test Output
```bash
cargo test -- --nocapture
```

### Test Coverage (Requires tarpaulin)
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Test Strategy

### 1. Unit Tests
- Test public methods of each service
- Use in-memory DB for high-speed execution
- Cover both success and error cases

### 2. Integration Tests
- Test workflows combining multiple services
- Use actual JJ repositories (managed by tempfile)
- Cover end-to-end scenarios

### 3. Test Data Management
- Use independent DB instances for each test
- Manage file system state with temporary directories (tempfile)
- Automatic cleanup after tests

## Quality Metrics

### Coverage
- ✅ **Models**: 100% - All enum conversion methods
- ✅ **Database**: 100% - All public methods
- ✅ **TaskService**: 100% - All public methods
- ✅ **TodoService**: 100% - All public methods
- ✅ **LinkService/ScrapService**: 100% - All public methods
- ✅ **RepoService**: 100% - All public methods
- ✅ **WorkspaceService**: 100% - All public methods
- ⚠️ **CommandHandler**: Partial - LLM help only

### Error Handling
Tested in all services:
- Access to non-existent resources
- Invalid input data
- Duplicate data handling
- Business rule violations (e.g., operations on archived tasks)

### Edge Cases
- Empty input
- Boundary values
- Data containing special characters
- Multiple concurrent operations

## Future Improvements

### 1. CommandHandler Test Expansion
Currently, CommandHandler is indirectly tested via integration tests, but consider adding direct unit tests for each command handler.

### 2. Performance Testing
Add performance tests for handling large numbers of tasks/TODOs.

### 3. Concurrency Testing
Add tests for when multiple operations are executed simultaneously.

### 4. Error Message Verification
Add tests to verify the content of messages returned in error cases.

## Maintenance

### When Adding New Features
1. Always add tests for new public methods
2. Cover both success and error cases
3. Verify actual workflows with integration tests

### When Tests Fail
1. Check the log of the failed test
2. Check the relevant code changes
3. Determine if the test or the implementation is correct
4. Fix the test or implementation as necessary

### When Refactoring
1. Ensure all tests continue to pass
2. Refactor test code as well
3. Commonize duplicate test code
