# Installation Guide

## Quick Start

### 1. Build the Project

```bash
cargo build --release
```

### 2. Install the Binary

```bash
# Copy to a location in your PATH
sudo cp target/release/task2habitica /usr/local/bin/

# Or for single-user installation
cp target/release/task2habitica ~/.local/bin/
```

### 3. Install Hook Scripts

```bash
# Create hooks directory if it doesn't exist
mkdir -p ~/.task/hooks

# Copy hook scripts
cp hooks/* ~/.task/hooks/

# Make sure they're executable
chmod +x ~/.task/hooks/*.task2habitica
```

### 4. Configure Taskwarrior

Add to your `~/.taskrc`:

```
# Habitica credentials
habitica.user_id=YOUR_USER_ID_HERE
habitica.api_key=YOUR_API_KEY_HERE

# Required UDAs
uda.habitica_uuid.label=Habitica UUID
uda.habitica_uuid.type=string

uda.habitica_difficulty.label=Habitica Difficulty
uda.habitica_difficulty.type=string
uda.habitica_difficulty.values=trivial,easy,medium,hard

uda.habitica_task_type.label=Habitica Task Type
uda.habitica_task_type.type=string
uda.habitica_task_type.values=daily,todo
```

### 5. Get Your Habitica Credentials

1. Log in to https://habitica.com
2. Go to Settings → API
3. Copy your User ID and API Token
4. Replace `YOUR_USER_ID_HERE` and `YOUR_API_KEY_HERE` in `.taskrc`

### 6. Test the Installation

```bash
# Check that the binary is accessible
task2habitica --version

# Add a test task
task add "Test task from Taskwarrior"

# Check if it appears on Habitica
# Run a manual sync
task2habitica sync
```

## Optional Configuration

### Task Notes

By default, task notes are stored in `~/.task/notes/`. To customize:

```
rc.tasknote.location=~/Documents/task-notes/
rc.tasknote.prefix=[note]
rc.tasknote.extension=.md
```

### Verbose Output

For debugging, use the `--verbose` flag:

```bash
task2habitica sync --verbose
```

## Troubleshooting

### "task2habitica: command not found"

Make sure `/usr/local/bin` (or `~/.local/bin`) is in your PATH:

```bash
echo $PATH
```

If not, add to your `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$PATH:/usr/local/bin"
# or for ~/.local/bin
export PATH="$PATH:$HOME/.local/bin"
```

### Hooks Not Running

Check permissions:

```bash
ls -la ~/.task/hooks/
```

They should be executable (`-rwxr-xr-x`).

### Compilation Errors

Make sure you have Rust 1.70 or higher:

```bash
rustc --version
```

If not, update Rust:

```bash
rustup update
```

### Cannot Find Taskwarrior

task2habitica needs Taskwarrior 3.4.2+ installed:

```bash
task --version
```

### "Failed to serialize/deserialize JSON" Error

This error usually means you're running an older version of the binary that doesn't support Taskwarrior's date format. Make sure to:

1. Rebuild the project: `cargo build --release`
2. Reinstall the binary: `sudo cp target/release/task2habitica /usr/local/bin/`
3. Verify the version: `task2habitica --version` (should show 0.1.0)

If the error persists, check which binary the hooks are using:

```bash
cat ~/.task/hooks/on-add.task2habitica
```

Make sure it points to the correct binary location.

## Uninstallation

```bash
# Remove binary
sudo rm /usr/local/bin/task2habitica

# Remove hooks
rm ~/.task/hooks/*.task2habitica

# Remove configuration (optional)
# Edit ~/.taskrc and remove habitica.* and uda.habitica_* entries

# Remove notes (optional)
rm -rf ~/.task/notes/

# Remove cache
rm ~/.task/cached_habitica_stats.json
```
