# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_track_global_optspecs
	string join \n h/help
end

function __fish_track_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_track_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_track_using_subcommand
	set -l cmd (__fish_track_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c track -n "__fish_track_needs_command" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_needs_command" -f -a "new" -d 'Create a new task and switch to it'
complete -c track -n "__fish_track_needs_command" -f -a "list" -d 'List tasks'
complete -c track -n "__fish_track_needs_command" -f -a "switch" -d 'Switch to a different task'
complete -c track -n "__fish_track_needs_command" -f -a "status" -d 'Show detailed information about the current task'
complete -c track -n "__fish_track_needs_command" -f -a "desc" -d 'View or set task description'
complete -c track -n "__fish_track_needs_command" -f -a "ticket" -d 'Link a ticket to a task'
complete -c track -n "__fish_track_needs_command" -f -a "archive" -d 'Archive a task'
complete -c track -n "__fish_track_needs_command" -f -a "todo" -d 'TODO management'
complete -c track -n "__fish_track_needs_command" -f -a "link" -d 'Link management'
complete -c track -n "__fish_track_needs_command" -f -a "scrap" -d 'Scrap (work notes) management'
complete -c track -n "__fish_track_needs_command" -f -a "sync" -d 'Sync repositories and setup task branches'
complete -c track -n "__fish_track_needs_command" -f -a "repo" -d 'Repository management'
complete -c track -n "__fish_track_needs_command" -f -a "alias" -d 'Task alias management'
complete -c track -n "__fish_track_needs_command" -f -a "llm-help" -d 'Show help optimized for LLM agents'
complete -c track -n "__fish_track_needs_command" -f -a "completion" -d 'Generate shell completion script'
complete -c track -n "__fish_track_needs_command" -f -a "webui" -d 'Start web-based user interface'
complete -c track -n "__fish_track_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand new" -s d -l description -d 'Task description' -r
complete -c track -n "__fish_track_using_subcommand new" -s t -l ticket -d 'Ticket ID (e.g., PROJ-123, owner/repo/456)' -r
complete -c track -n "__fish_track_using_subcommand new" -l ticket-url -d 'Ticket URL' -r
complete -c track -n "__fish_track_using_subcommand new" -l template -d 'Template task reference (ID, ticket, or alias) to copy TODOs from' -r
complete -c track -n "__fish_track_using_subcommand new" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand list" -s a -l all -d 'Include archived tasks'
complete -c track -n "__fish_track_using_subcommand list" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand switch" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand status" -s j -l json -d 'Output in JSON format'
complete -c track -n "__fish_track_using_subcommand status" -s a -l all -d 'Show all scraps'
complete -c track -n "__fish_track_using_subcommand status" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand desc" -s t -l task -d 'Target task ID (defaults to current task)' -r
complete -c track -n "__fish_track_using_subcommand desc" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand ticket" -l task -d 'Target task ID (defaults to current task)' -r
complete -c track -n "__fish_track_using_subcommand ticket" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand archive" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "add" -d 'Add a new TODO'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "list" -d 'List TODOs'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "update" -d 'Update TODO status'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "done" -d 'Complete a TODO (merges worktree if exists)'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "delete" -d 'Delete a TODO'
complete -c track -n "__fish_track_using_subcommand todo; and not __fish_seen_subcommand_from add list update done delete help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from add" -s w -l worktree -d 'Create worktrees for this TODO'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from done" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from delete" -s f -l force -d 'Skip confirmation prompt'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a new TODO'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "list" -d 'List TODOs'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "update" -d 'Update TODO status'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "done" -d 'Complete a TODO (merges worktree if exists)'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Delete a TODO'
complete -c track -n "__fish_track_using_subcommand todo; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand link; and not __fish_seen_subcommand_from add list delete help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand link; and not __fish_seen_subcommand_from add list delete help" -f -a "add" -d 'Add a new link'
complete -c track -n "__fish_track_using_subcommand link; and not __fish_seen_subcommand_from add list delete help" -f -a "list" -d 'List links'
complete -c track -n "__fish_track_using_subcommand link; and not __fish_seen_subcommand_from add list delete help" -f -a "delete" -d 'Delete a link'
complete -c track -n "__fish_track_using_subcommand link; and not __fish_seen_subcommand_from add list delete help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a new link'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from help" -f -a "list" -d 'List links'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Delete a link'
complete -c track -n "__fish_track_using_subcommand link; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand scrap; and not __fish_seen_subcommand_from add list help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand scrap; and not __fish_seen_subcommand_from add list help" -f -a "add" -d 'Add a new scrap (work note)'
complete -c track -n "__fish_track_using_subcommand scrap; and not __fish_seen_subcommand_from add list help" -f -a "list" -d 'List scraps'
complete -c track -n "__fish_track_using_subcommand scrap; and not __fish_seen_subcommand_from add list help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand scrap; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand scrap; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand scrap; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a new scrap (work note)'
complete -c track -n "__fish_track_using_subcommand scrap; and __fish_seen_subcommand_from help" -f -a "list" -d 'List scraps'
complete -c track -n "__fish_track_using_subcommand scrap; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand sync" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand repo; and not __fish_seen_subcommand_from add list remove help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand repo; and not __fish_seen_subcommand_from add list remove help" -f -a "add" -d 'Add a repository to the current task'
complete -c track -n "__fish_track_using_subcommand repo; and not __fish_seen_subcommand_from add list remove help" -f -a "list" -d 'List repositories'
complete -c track -n "__fish_track_using_subcommand repo; and not __fish_seen_subcommand_from add list remove help" -f -a "remove" -d 'Remove a repository'
complete -c track -n "__fish_track_using_subcommand repo; and not __fish_seen_subcommand_from add list remove help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from add" -s b -l base -d 'Base branch to use (defaults to current branch)' -r
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a repository to the current task'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "list" -d 'List repositories'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a repository'
complete -c track -n "__fish_track_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand alias; and not __fish_seen_subcommand_from set remove help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand alias; and not __fish_seen_subcommand_from set remove help" -f -a "set" -d 'Set an alias for the current task'
complete -c track -n "__fish_track_using_subcommand alias; and not __fish_seen_subcommand_from set remove help" -f -a "remove" -d 'Remove the alias from the current task'
complete -c track -n "__fish_track_using_subcommand alias; and not __fish_seen_subcommand_from set remove help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from set" -s t -l task -d 'Target task ID (defaults to current task)' -r
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from remove" -s t -l task -d 'Target task ID (defaults to current task)' -r
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set an alias for the current task'
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove the alias from the current task'
complete -c track -n "__fish_track_using_subcommand alias; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand llm-help" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand completion" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand webui" -s p -l port -d 'Port to listen on' -r
complete -c track -n "__fish_track_using_subcommand webui" -s o -l open -d 'Open browser automatically'
complete -c track -n "__fish_track_using_subcommand webui" -s h -l help -d 'Print help'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "new" -d 'Create a new task and switch to it'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "list" -d 'List tasks'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "switch" -d 'Switch to a different task'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "status" -d 'Show detailed information about the current task'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "desc" -d 'View or set task description'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "ticket" -d 'Link a ticket to a task'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "archive" -d 'Archive a task'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "todo" -d 'TODO management'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "link" -d 'Link management'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "scrap" -d 'Scrap (work notes) management'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "sync" -d 'Sync repositories and setup task branches'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "repo" -d 'Repository management'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "alias" -d 'Task alias management'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "llm-help" -d 'Show help optimized for LLM agents'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "completion" -d 'Generate shell completion script'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "webui" -d 'Start web-based user interface'
complete -c track -n "__fish_track_using_subcommand help; and not __fish_seen_subcommand_from new list switch status desc ticket archive todo link scrap sync repo alias llm-help completion webui help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from todo" -f -a "add" -d 'Add a new TODO'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from todo" -f -a "list" -d 'List TODOs'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from todo" -f -a "update" -d 'Update TODO status'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from todo" -f -a "done" -d 'Complete a TODO (merges worktree if exists)'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from todo" -f -a "delete" -d 'Delete a TODO'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from link" -f -a "add" -d 'Add a new link'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from link" -f -a "list" -d 'List links'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from link" -f -a "delete" -d 'Delete a link'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from scrap" -f -a "add" -d 'Add a new scrap (work note)'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from scrap" -f -a "list" -d 'List scraps'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "add" -d 'Add a repository to the current task'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "list" -d 'List repositories'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "remove" -d 'Remove a repository'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from alias" -f -a "set" -d 'Set an alias for the current task'
complete -c track -n "__fish_track_using_subcommand help; and __fish_seen_subcommand_from alias" -f -a "remove" -d 'Remove the alias from the current task'
