# Develop

```
cargo build && cargo run -- -help
```


# Install

```
cargo install --path .
```

# Usage

state is stored in file `~/.timelog-state

finished task records are stored in file `~/.timelog-record`

```
timelog start <task> # start a timer on a task

timelog pause # pause the timer

timelog resume # resume the timer

timelog status # show current task and timer status

timelog stop # stop the timer, record task to report file

timelog report <period> # display each task and total time for given period
```


