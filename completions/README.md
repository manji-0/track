# Shell Completion Installation Guide

This directory contains shell completion scripts for the `track` CLI tool.

## Quick Installation

### Bash

```bash
# Copy the completion script to your bash completions directory
sudo cp completions/track.bash /usr/share/bash-completion/completions/track

# Or for user-level installation:
mkdir -p ~/.local/share/bash-completion/completions
cp completions/track.bash ~/.local/share/bash-completion/completions/track

# Reload your shell or source the completion:
source ~/.local/share/bash-completion/completions/track
```

### Zsh

```bash
# Copy to a directory in your $fpath
# First, find a suitable directory:
echo $fpath

# Common locations:
# - /usr/local/share/zsh/site-functions
# - ~/.zsh/completions

# System-wide installation:
sudo cp completions/_track /usr/local/share/zsh/site-functions/_track

# Or user-level installation:
mkdir -p ~/.zsh/completions
cp completions/_track ~/.zsh/completions/_track

# Add to your ~/.zshrc if not already present:
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

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

If you prefer to generate the completion scripts yourself:

```bash
# Bash
track completion bash > ~/.local/share/bash-completion/completions/track

# Zsh
track completion zsh > ~/.zsh/completions/_track

# Fish
track completion fish > ~/.config/fish/completions/track.fish

# PowerShell
track completion powershell > ~/Documents/PowerShell/Scripts/_track.ps1
```

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

## Limitations

The current completion scripts provide **static completions** based on the CLI structure. They do not currently provide:

- Dynamic task ID/name suggestions for `track switch`
- Dynamic TODO ID suggestions for `track todo done/delete`
- Dynamic link ID suggestions for `track link delete`

These dynamic completions may be added in a future release.

## Contributing

If you'd like to enhance the completion scripts with dynamic data, please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.
