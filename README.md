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
```


## Modify state/record files

```bash
# Set custom record file path
export TIMELOG_RECORD_PATH="/path/to/custom/records.csv"

# Set custom state file path  
export TIMELOG_STATE_PATH="/path/to/custom/state.json"

# Run timelog with custom paths
timelog start "my task"
```