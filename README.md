# Develop

```bash
cargo build && cargo run -- -help
```


# Install

```bash
cargo install --path .
```

# Usage

state is stored in file `~/.timelog-state

finished task records are stored in file `~/.timelog-record`

```bash
timelog start <task> # start a timer on a task
timelog start <task> --project <project> # start a timer on a task in a specific project

timelog pause # pause the timer

timelog resume # resume the timer

timelog status # show current task and timer status

timelog stop # stop the timer, record task to report file

timelog report <period> # display each task and total time for given period
timelog report <period> --project <project> # filter report by project

timelog amend --date <YYYY-MM-DD> --task <pattern> [options] # amend existing records
```

## Amending Records

The `amend` command allows you to modify existing time records. You can change the task name, duration, or project for any previously recorded entry.

### Basic Usage

```bash
# Change task name
timelog amend --date 2024-01-15 --task "old task" --new-task "updated task name"

# Change duration (in minutes)
timelog amend --date 2024-01-15 --task "coding" --new-duration 120

# Change project
timelog amend --date 2024-01-15 --task "meeting" --new-project "newproject"

# Remove project (set to none)
timelog amend --date 2024-01-15 --task "task" --new-project ""

# Change multiple fields at once
timelog amend --date 2024-01-15 --task "old" --new-task "new task" --new-duration 90 --new-project "proj"

# Preview changes without applying them
timelog amend --date 2024-01-15 --task "task" --new-task "updated" --dry-run
```

### Notes

- The `--task` parameter performs a partial match on task names
- If multiple records match, you'll need to be more specific with the task pattern
- All changes are validated (e.g., duration must be positive)
- Use `--dry-run` to preview changes before applying them


## Modify state/record files

```bash
# Set custom record file path
export TIMELOG_RECORD_PATH="/path/to/custom/records.csv"

# Set custom state file path  
export TIMELOG_STATE_PATH="/path/to/custom/state.json"

# Run timelog with custom paths
timelog start "my task"
```