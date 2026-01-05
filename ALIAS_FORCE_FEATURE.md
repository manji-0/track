# Alias Force Overwrite Feature

## Summary

Added a `--force` flag to the `track alias set` command that allows users to overwrite an existing alias on another task. When the flag is used, the alias is removed from the previous task and set on the new task.

## Changes Made

### 1. CLI Definition (`src/cli/mod.rs`)
- Added `force: bool` field to `AliasCommands::Set` variant
- Added `#[arg(short, long)]` attribute for the `--force` flag

### 2. Command Handler (`src/cli/handler.rs`)
- Updated `handle_alias` to pass the `force` parameter to `set_alias`
- Updated LLM help text to document the new flag

### 3. Service Layer (`src/services/task_service.rs`)
- Modified `set_alias` method signature to accept `force: bool` parameter
- When `force=true` and alias exists on another task:
  - Removes the alias from the existing task
  - Sets the alias on the new task
- When `force=false` and alias exists on another task:
  - Returns an error with a helpful message suggesting to use `--force`
- Added comprehensive test `test_set_alias_force_overwrite` to verify the functionality
- Updated all existing tests to pass `false` for the force parameter

### 4. Documentation Updates
- **README.md**: Added command reference for `track alias set <alias> --force`
- **docs/LLM_HELP_DESIGN.md**: Added documentation for the force flag
- **docs/USAGE_EXAMPLES.md**: Added detailed explanation and example of the force flag usage
- **src/cli/handler.rs**: Updated the `llm-help` command output

## Usage Examples

### Without Force (Default Behavior)
```bash
# Task 1 has alias "daily-report"
track switch 2
track alias set daily-report
# Error: Alias 'daily-report' is already in use by task #1. Use --force to overwrite.
```

### With Force Flag
```bash
# Task 1 has alias "daily-report"
track switch 2
track alias set daily-report --force
# Success: Alias removed from task #1 and set on task #2
```

## Testing

All tests pass successfully:
- Existing alias tests updated to use the new signature
- New test added to verify force overwrite functionality
- Full test suite passes (119 tests)

## Backward Compatibility

The change is backward compatible:
- The `--force` flag is optional (defaults to `false`)
- Without the flag, behavior is identical to before (returns error on duplicate)
- Error message now includes helpful hint to use `--force`
