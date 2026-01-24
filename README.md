# task2habitica-rs

Bidirectional sync tool between [Taskwarrior](https://taskwarrior.org) and [Habitica](https://habitica.com).

## Features

- ✅ Bidirectional sync between Taskwarrior and Habitica
- ✅ Automatic task creation, updates, and completion tracking
- ✅ Task difficulty mapping (trivial/easy/medium/hard)
- ✅ Support for todos and dailies

## Requirements

- Rust 1.70 or higher
- Taskwarrior 3.4.2 or higher
- Habitica account

## Installation

### Using Cargo (Recommended)

```bash
cargo install task2habitica
```

### From Source

```bash
# Clone the repository
git clone https://github.com/mainframev/task2habitica-rs.git

# Build the release binary
cargo build --release

# Install the binary
cp target/release/task2habitica /usr/local/bin/

# Install the hook scripts
mkdir -p ~/.task/hooks
cp hooks/* ~/.task/hooks/
chmod +x ~/.task/hooks/*.task2habitica
```

## Configuration

### 1. Add Habitica Credentials

You can configure your Habitica credentials using either environment variables or your `.taskrc` file.
Environment variables take precedence if both are set.

#### Environment Variables (Recommended)

```bash
export HABITICA_USER_ID=YOUR_USER_ID
export HABITICA_API_KEY=YOUR_API_KEY
```

#### .taskrc

Add your Habitica user ID and API key to your `taskrc` file:

```
habitica.user_id=YOUR_USER_ID
habitica.api_key=YOUR_API_KEY
```

You can find these in your Habitica account settings under _Site Data tab_.

### 2. Add Required UDAs to .taskrc

Add the following User Defined Attributes (UDAs) to your `taskrc`:

```
uda.habitica_uuid.label=Habitica UUID
uda.habitica_uuid.type=string

uda.habitica_difficulty.label=Habitica Difficulty
uda.habitica_difficulty.type=string
uda.habitica_difficulty.values=trivial,easy,medium,hard

uda.habitica_task_type.label=Habitica Task Type
uda.habitica_task_type.type=string
uda.habitica_task_type.values=daily,todo
```

### 3. Optional: Configure Task Notes

By default, task notes are stored in `~/.task/notes/`. You can customize this:

```
rc.tasknote.location=~/.task/notes/
rc.tasknote.prefix=[tasknote]
rc.tasknote.extension=.txt
```

## Usage

### Automatic Sync (via Hooks)

Once installed, the hooks will automatically sync your tasks:

- **on-add**: When you add a task in Taskwarrior, it's created on Habitica
- **on-modify**: When you modify a task, changes are synced to Habitica
- **on-exit**: Displays stat changes (HP, MP, Exp, Gold) when Taskwarrior exits

Example:

```bash
task add "Buy groceries"
task 1 done
# Stats will be displayed on exit
```

### Manual Sync

To manually sync all tasks:

```bash
task2habitica sync
```

This is useful when:

- You've added tasks on Habitica and want to import them
- Initial setup to sync existing tasks
- Recovering from sync issues

Use `--verbose` flag for detailed output:

```bash
task2habitica sync --verbose
```

### Task Difficulty

Set task difficulty using the `habitica_difficulty` UDA:

```bash
task add "Easy task" habitica_difficulty:easy
task add "Hard boss fight" habitica_difficulty:hard
```

Difficulty levels:

- `trivial`: 0.1 priority in Habitica
- `easy`: 1.0 priority (default)
- `medium`: 1.5 priority
- `hard`: 2.0 priority

### Task Types

Specify task type using the `habitica_task_type` UDA:

```bash
task add "Daily exercise" habitica_task_type:daily
task add "One-time task" habitica_task_type:todo
```

### Task Notes

Task notes from Habitica are stored as separate files in `~/.task/notes/`:

```bash
# Notes are automatically synced
task 1 annotate "This is an annotation, not a note"
# The Habitica notes field will be saved to ~/.task/notes/<uuid>.txt
```

## How It Works

### Bidirectional Sync

The sync process:

1. **Taskwarrior-only tasks**: Pushed to Habitica with a new Habitica UUID
2. **Habitica-only tasks**: Imported to Taskwarrior
3. **Tasks on both sides**:
   - If identical, no action taken
   - If different, most recently modified version wins
   - Modification timestamps are compared to resolve conflicts

### Status Mapping

| Taskwarrior Status | Habitica Status | Sync Behavior              |
| ------------------ | --------------- | -------------------------- |
| pending            | pending         | Synced                     |
| waiting            | pending         | Synced                     |
| completed          | completed       | Synced, scored on Habitica |
| deleted            | (deleted)       | Not synced                 |
| recurring          | (template)      | Not synced                 |

## Support

- Issues: https://github.com/mainframev/task2habitica-rs/issues
- Habitica: https://habitica.com
- Taskwarrior: https://taskwarrior.org
