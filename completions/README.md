# Shell Completion Installation Guide

This directory contains shell completion scripts for the `track` CLI tool.

## Quick Installation

**Recommended:** Install dynamic completions for the best experience with real-time data.

### Bash (Dynamic)

```bash
# User-level installation (recommended):
mkdir -p ~/.local/share/bash-completion/completions
track completion bash --dynamic > ~/.local/share/bash-completion/completions/track

# Reload your shell or source the completion:
source ~/.local/share/bash-completion/completions/track
```

**Alternative (Static):** Use `track completion bash` (without `--dynamic`) for static completions.

### Zsh (Dynamic)

```bash
# User-level installation (recommended):
mkdir -p ~/.zsh/completions
track completion zsh --dynamic > ~/.zsh/completions/_track

# Add to your ~/.zshrc if not already present:
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit

# Reload completions:
rm -f ~/.zcompdump*
exec zsh
```

**Alternative (Static):** Use `track completion zsh` (without `--dynamic`) for static completions.

### Fish

```bash
# Copy to fish completions directory
mkdir -p ~/.config/fish/completions
cp completions/track.fish ~/.config/fish/completions/track.fish

# Fish will automatically load completions on next shell start
```

### PowerShell

```powershell
# Add to your PowerShell profile
# Find your profile location:
$PROFILE

# Create profile if it doesn't exist:
if (!(Test-Path -Path $PROFILE)) {
  New-Item -ItemType File -Path $PROFILE -Force
}

# Add the completion script to your profile:
Add-Content $PROFILE ". /path/to/track/completions/_track.ps1"

# Reload your profile:
. $PROFILE
```

## Generating Completions Manually

### Static Completions (Basic)

The `track completion` command generates **static completions** (no dynamic data):

```bash
# Bash (static)
track completion bash > ~/.local/share/bash-completion/completions/track

# Zsh (static)
track completion zsh > ~/.zsh/completions/_track

# Fish (static)
track completion fish > ~/.config/fish/completions/track.fish

# PowerShell (static)
track completion powershell > ~/Documents/PowerShell/Scripts/_track.ps1
```

**Note:** Static completions provide command/subcommand/flag completion but **do not show dynamic data** (task IDs, TODO IDs, etc.).

### Dynamic Completions (Recommended)

For the best experience with **dynamic data completion**, use the pre-built scripts from the repository:

```bash
# Bash (dynamic) - Recommended
cp completions/track.bash.dynamic ~/.local/share/bash-completion/completions/track
source ~/.local/share/bash-completion/completions/track

# Zsh (dynamic) - Recommended
mkdir -p ~/.zsh/completions
cp completions/_track.dynamic ~/.zsh/completions/_track
# Add to ~/.zshrc if not present:
# fpath=(~/.zsh/completions $fpath)
# autoload -Uz compinit && compinit
```

Dynamic completions show real-time data from your database when you press TAB.

## Verifying Installation

After installation, start a new shell session and try:

```bash
track <TAB>        # Should show available commands
track todo <TAB>   # Should show todo subcommands
track switch <TAB> # Should complete task references
```

## Troubleshooting

### Bash: Completions not working

1. Ensure `bash-completion` package is installed:
   ```bash
   # Ubuntu/Debian
   sudo apt install bash-completion
   
   # macOS
   brew install bash-completion@2
   ```

2. Verify bash-completion is enabled in your `~/.bashrc`:
   ```bash
   if [ -f /etc/bash_completion ]; then
       . /etc/bash_completion
   fi
   ```

### Zsh: Completions not working

1. Ensure `compinit` is called in your `~/.zshrc`:
   ```bash
   autoload -Uz compinit && compinit
   ```

2. Clear the completion cache:
   ```bash
   rm -f ~/.zcompdump*
   compinit
   ```

3. Verify the completion file is in your `$fpath`:
   ```bash
   echo $fpath | grep -o '[^ ]*completions[^ ]*'
   ```

### Fish: Completions not working

1. Verify the completion file location:
   ```bash
   ls ~/.config/fish/completions/track.fish
   ```

2. Reload fish completions:
   ```bash
   fish_update_completions
   ```

## Features

The completion scripts provide:

- **Command completion**: All main commands (new, list, switch, status, etc.)
- **Subcommand completion**: Nested commands (todo add, link delete, etc.)
- **Flag completion**: All available flags and options
- **Argument hints**: Contextual hints for required arguments

### Dynamic Completions (Zsh & Bash)

Both zsh and bash completion scripts include **dynamic completions** that show real data from your track database:

- **`track switch <TAB>`**: Shows actual task IDs and names
- **`track todo done <TAB>`**: Shows pending TODO IDs and content
- **`track todo update <TAB>`**: Shows pending TODO IDs and content
- **`track todo delete <TAB>`**: Shows pending TODO IDs and content
- **`track link delete <TAB>`**: Shows link IDs and titles
- **`track repo remove <TAB>`**: Shows repository IDs and paths
- **`track archive <TAB>`**: Shows task IDs and names
- **`track new --template <TAB>`**: Shows task IDs for template selection

These dynamic completions are powered by the hidden `track _complete` command which queries the database in real-time.

### Completion Versions

#### Zsh

Two versions of the zsh completion script are available:

1. **`_track`** (default): Includes dynamic completions - recommended for most users
2. **`_track.static`**: Static completions only (generated by `track completion zsh`)

#### Bash

Two versions of the bash completion script are available:

1. **`track.bash.dynamic`**: Includes dynamic completions - recommended for most users
2. **`track.bash`**: Static completions only (generated by `track completion bash`)

To use the dynamic bash completions:

```bash
# Copy the dynamic version
cp completions/track.bash.dynamic ~/.local/share/bash-completion/completions/track
source ~/.local/share/bash-completion/completions/track
```

The dynamic version provides a better user experience by showing actual data, while the static version is faster and doesn't require database access.

## Performance Note

Dynamic completions query the database each time you press TAB. For most users, this is instantaneous. If you have a very large number of tasks (100+) and experience slowness, you can use the static completion script instead.

## Contributing

If you'd like to enhance the completion scripts or add support for other shells, please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.
