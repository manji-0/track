
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'track' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'track'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'track' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('new', 'new', [CompletionResultType]::ParameterValue, 'Create a new task and switch to it')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List tasks')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch to a different task')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show detailed information about the current task')
            [CompletionResult]::new('desc', 'desc', [CompletionResultType]::ParameterValue, 'View or set task description')
            [CompletionResult]::new('ticket', 'ticket', [CompletionResultType]::ParameterValue, 'Link a ticket to a task')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a task')
            [CompletionResult]::new('todo', 'todo', [CompletionResultType]::ParameterValue, 'TODO management')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Link management')
            [CompletionResult]::new('scrap', 'scrap', [CompletionResultType]::ParameterValue, 'Scrap (work notes) management')
            [CompletionResult]::new('sync', 'sync', [CompletionResultType]::ParameterValue, 'Sync repositories and setup task branches')
            [CompletionResult]::new('repo', 'repo', [CompletionResultType]::ParameterValue, 'Repository management')
            [CompletionResult]::new('alias', 'alias', [CompletionResultType]::ParameterValue, 'Task alias management')
            [CompletionResult]::new('llm-help', 'llm-help', [CompletionResultType]::ParameterValue, 'Show help optimized for LLM agents')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion script')
            [CompletionResult]::new('webui', 'webui', [CompletionResultType]::ParameterValue, 'Start web-based user interface')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;new' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Task description')
            [CompletionResult]::new('--description', '--description', [CompletionResultType]::ParameterName, 'Task description')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Ticket ID (e.g., PROJ-123, owner/repo/456)')
            [CompletionResult]::new('--ticket', '--ticket', [CompletionResultType]::ParameterName, 'Ticket ID (e.g., PROJ-123, owner/repo/456)')
            [CompletionResult]::new('--ticket-url', '--ticket-url', [CompletionResultType]::ParameterName, 'Ticket URL')
            [CompletionResult]::new('--template', '--template', [CompletionResultType]::ParameterName, 'Template task reference (ID, ticket, or alias) to copy TODOs from')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;list' {
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Include archived tasks')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Include archived tasks')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;switch' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;status' {
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Output in JSON format')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Show all scraps')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Show all scraps')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;desc' {
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('--task', '--task', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;ticket' {
            [CompletionResult]::new('--task', '--task', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;archive' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new TODO')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List TODOs')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update TODO status')
            [CompletionResult]::new('done', 'done', [CompletionResultType]::ParameterValue, 'Complete a TODO (merges worktree if exists)')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a TODO')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;todo;add' {
            [CompletionResult]::new('-w', '-w', [CompletionResultType]::ParameterName, 'Create worktrees for this TODO')
            [CompletionResult]::new('--worktree', '--worktree', [CompletionResultType]::ParameterName, 'Create worktrees for this TODO')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo;list' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo;update' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo;done' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo;delete' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Skip confirmation prompt')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Skip confirmation prompt')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;todo;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new TODO')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List TODOs')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update TODO status')
            [CompletionResult]::new('done', 'done', [CompletionResultType]::ParameterValue, 'Complete a TODO (merges worktree if exists)')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a TODO')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;todo;help;add' {
            break
        }
        'track;todo;help;list' {
            break
        }
        'track;todo;help;update' {
            break
        }
        'track;todo;help;done' {
            break
        }
        'track;todo;help;delete' {
            break
        }
        'track;todo;help;help' {
            break
        }
        'track;link' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new link')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List links')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a link')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;link;add' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;link;list' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;link;delete' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;link;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new link')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List links')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a link')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;link;help;add' {
            break
        }
        'track;link;help;list' {
            break
        }
        'track;link;help;delete' {
            break
        }
        'track;link;help;help' {
            break
        }
        'track;scrap' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new scrap (work note)')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List scraps')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;scrap;add' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;scrap;list' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;scrap;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new scrap (work note)')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List scraps')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;scrap;help;add' {
            break
        }
        'track;scrap;help;list' {
            break
        }
        'track;scrap;help;help' {
            break
        }
        'track;sync' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;repo' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a repository to the current task')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List repositories')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a repository')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;repo;add' {
            [CompletionResult]::new('-b', '-b', [CompletionResultType]::ParameterName, 'Base branch to use (defaults to current branch)')
            [CompletionResult]::new('--base', '--base', [CompletionResultType]::ParameterName, 'Base branch to use (defaults to current branch)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;repo;list' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;repo;remove' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;repo;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a repository to the current task')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List repositories')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a repository')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;repo;help;add' {
            break
        }
        'track;repo;help;list' {
            break
        }
        'track;repo;help;remove' {
            break
        }
        'track;repo;help;help' {
            break
        }
        'track;alias' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set an alias for the current task')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove the alias from the current task')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;alias;set' {
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('--task', '--task', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;alias;remove' {
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('--task', '--task', [CompletionResultType]::ParameterName, 'Target task ID (defaults to current task)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;alias;help' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set an alias for the current task')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove the alias from the current task')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;alias;help;set' {
            break
        }
        'track;alias;help;remove' {
            break
        }
        'track;alias;help;help' {
            break
        }
        'track;llm-help' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;completion' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;webui' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Port to listen on')
            [CompletionResult]::new('--port', '--port', [CompletionResultType]::ParameterName, 'Port to listen on')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Open browser automatically')
            [CompletionResult]::new('--open', '--open', [CompletionResultType]::ParameterName, 'Open browser automatically')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'track;help' {
            [CompletionResult]::new('new', 'new', [CompletionResultType]::ParameterValue, 'Create a new task and switch to it')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List tasks')
            [CompletionResult]::new('switch', 'switch', [CompletionResultType]::ParameterValue, 'Switch to a different task')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show detailed information about the current task')
            [CompletionResult]::new('desc', 'desc', [CompletionResultType]::ParameterValue, 'View or set task description')
            [CompletionResult]::new('ticket', 'ticket', [CompletionResultType]::ParameterValue, 'Link a ticket to a task')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a task')
            [CompletionResult]::new('todo', 'todo', [CompletionResultType]::ParameterValue, 'TODO management')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Link management')
            [CompletionResult]::new('scrap', 'scrap', [CompletionResultType]::ParameterValue, 'Scrap (work notes) management')
            [CompletionResult]::new('sync', 'sync', [CompletionResultType]::ParameterValue, 'Sync repositories and setup task branches')
            [CompletionResult]::new('repo', 'repo', [CompletionResultType]::ParameterValue, 'Repository management')
            [CompletionResult]::new('alias', 'alias', [CompletionResultType]::ParameterValue, 'Task alias management')
            [CompletionResult]::new('llm-help', 'llm-help', [CompletionResultType]::ParameterValue, 'Show help optimized for LLM agents')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion script')
            [CompletionResult]::new('webui', 'webui', [CompletionResultType]::ParameterValue, 'Start web-based user interface')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'track;help;new' {
            break
        }
        'track;help;list' {
            break
        }
        'track;help;switch' {
            break
        }
        'track;help;status' {
            break
        }
        'track;help;desc' {
            break
        }
        'track;help;ticket' {
            break
        }
        'track;help;archive' {
            break
        }
        'track;help;todo' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new TODO')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List TODOs')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update TODO status')
            [CompletionResult]::new('done', 'done', [CompletionResultType]::ParameterValue, 'Complete a TODO (merges worktree if exists)')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a TODO')
            break
        }
        'track;help;todo;add' {
            break
        }
        'track;help;todo;list' {
            break
        }
        'track;help;todo;update' {
            break
        }
        'track;help;todo;done' {
            break
        }
        'track;help;todo;delete' {
            break
        }
        'track;help;link' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new link')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List links')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a link')
            break
        }
        'track;help;link;add' {
            break
        }
        'track;help;link;list' {
            break
        }
        'track;help;link;delete' {
            break
        }
        'track;help;scrap' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new scrap (work note)')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List scraps')
            break
        }
        'track;help;scrap;add' {
            break
        }
        'track;help;scrap;list' {
            break
        }
        'track;help;sync' {
            break
        }
        'track;help;repo' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a repository to the current task')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List repositories')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a repository')
            break
        }
        'track;help;repo;add' {
            break
        }
        'track;help;repo;list' {
            break
        }
        'track;help;repo;remove' {
            break
        }
        'track;help;alias' {
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set an alias for the current task')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove the alias from the current task')
            break
        }
        'track;help;alias;set' {
            break
        }
        'track;help;alias;remove' {
            break
        }
        'track;help;llm-help' {
            break
        }
        'track;help;completion' {
            break
        }
        'track;help;webui' {
            break
        }
        'track;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
