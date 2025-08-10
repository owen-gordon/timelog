use chrono::Utc;
use clap::Parser;
use std::fs;
use timelog::*;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { task, project } => {
            if state_path().exists() {
                die("a task is already in progress; run `timelog pause` or `timelog stop`");
            }

            let state = State {
                timestamp: Utc::now(),
                task: task.to_string(),
                active: true,
                project: project.clone(),
            };
            if let Err(e) = save_state(&state) {
                die(&e);
            }

            let project_info = match project {
                Some(p) => format!(" in project {}", emph(p)),
                None => String::new(),
            };
            info(&format!("started {}{}", emph(task), project_info));
        }

        Commands::Pause => {
            if !state_path().exists() {
                die("no active task to pause");
            }

            let state = match load_state() {
                Ok(s) => s,
                Err(e) => die(&e),
            };
            if !state.active {
                die("task is already paused; use `timelog resume`");
            }

            let elapsed = Utc::now() - state.timestamp;
            let epoch = chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap();
            let timestamp = epoch + elapsed;

            let paused_state = State {
                timestamp,
                task: state.task.clone(),
                active: false,
                project: state.project.clone(),
            };
            if let Err(e) = save_state(&paused_state) {
                die(&e);
            }

            info(&format!(
                "paused {}  (elapsed {})",
                emph(&state.task),
                fmt_hms_ms(elapsed.num_milliseconds()),
            ));
        }

        Commands::Resume => {
            if !state_path().exists() {
                die("no paused task to resume");
            }

            let state = match load_state() {
                Ok(s) => s,
                Err(e) => die(&e),
            };
            if state.active {
                die("task is already running");
            }

            let epoch = chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap();
            let elapsed = state.timestamp - epoch;
            let timestamp = Utc::now() - elapsed;

            let active_state = State {
                timestamp,
                task: state.task.clone(),
                active: true,
                project: state.project.clone(),
            };
            if let Err(e) = save_state(&active_state) {
                die(&e);
            }

            info(&format!("resumed {}", emph(&state.task)));
        }

        Commands::Stop => {
            if !state_path().exists() {
                die("no task to stop");
            }

            let state = match load_state() {
                Ok(s) => s,
                Err(e) => die(&e),
            };

            let elapsed = if state.active {
                Utc::now() - state.timestamp
            } else {
                let epoch = chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap();
                state.timestamp - epoch
            };

            let record = Record {
                task: state.task.clone(),
                duration_ms: elapsed.num_milliseconds(),
                date: Utc::now().naive_local().date(),
                project: state.project.clone(),
            };

            if let Err(e) = save_record(&record) {
                die(&e);
            }

            if let Err(e) = delete_state() {
                die(&e);
            }

            let project_info = match &record.project {
                Some(p) => format!(" in project {}", emph(p)),
                None => String::new(),
            };
            info(&format!(
                "recorded {}{}  {} on {}",
                emph(&record.task),
                project_info,
                fmt_hms_ms(record.duration_ms),
                record.date,
            ));
        }

        Commands::Report { period, project } => {
            let records = match load_records() {
                Ok(r) => r,
                Err(e) => die(&e),
            };

            let today = Utc::now().date_naive();
            let (start, end) = period_range(period.clone(), today);

            let mut filtered: Vec<Record> = records
                .into_iter()
                .filter(|x| x.date >= start && x.date <= end)
                .filter(|x| match project {
                    Some(p) => x.project.as_ref() == Some(p),
                    None => true,
                })
                .collect();

            if filtered.is_empty() {
                warn("no records in selected period");
                return;
            }

            // sort by date, then task
            filtered.sort_by_key(|r| (r.date, r.task.clone()));

            print_report(period.clone(), start, end, &filtered, project);
        }

        Commands::Status => {
            if !state_path().exists() {
                die("no task to provide status");
            }

            let state = match load_state() {
                Ok(s) => s,
                Err(e) => die(&e),
            };

            let epoch = chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap();

            // If active, elapsed = now - started_at; if paused, elapsed = stored
            let (elapsed_ms, since_ts, _status_str) = if state.active {
                let e = (Utc::now() - state.timestamp).num_milliseconds();
                (clamp_nonneg(e), state.timestamp, "active")
            } else {
                let e = (state.timestamp - epoch).num_milliseconds();
                (clamp_nonneg(e), state.timestamp, "paused")
            };

            // Pretty, concise status lines
            let project_info = match &state.project {
                Some(p) => format!(" in project {}", emph(p)),
                None => String::new(),
            };

            if state.active {
                // e.g., "active 00:42:10.123 since 2025-08-08T17:20:11Z  —  task: compile in project myproject"
                info(&format!(
                    "{}  {}  since {}  —  task: {}{}",
                    emph("active"),
                    fmt_hms_ms(elapsed_ms),
                    fmt_ts(since_ts),
                    emph(&state.task),
                    project_info,
                ));
            } else {
                // When paused, `since_ts` is the pause timestamp encoded in state.timestamp
                info(&format!(
                    "{}  accumulated {}  —  task: {}{}",
                    emph("paused"),
                    fmt_hms_ms(elapsed_ms),
                    emph(&state.task),
                    project_info,
                ));
            }
        }

        Commands::Upload {
            plugin,
            period,
            dry_run,
            list_plugins,
        } => {
            if *list_plugins {
                let plugins = discover_plugins();
                if plugins.is_empty() {
                    info("No plugins found");
                    info(&format!(
                        "Place plugin scripts in: {}",
                        plugin_dir().display()
                    ));
                    info("Plugin scripts should be named 'timelog-<name>' and be executable");
                } else {
                    info("Available plugins:");
                    for p in plugins {
                        println!("  • {p}");
                    }
                }
                return;
            }

            // Load records for the specified period
            let records = match load_records() {
                Ok(r) => r,
                Err(e) => die(&e),
            };

            let period = period.as_ref().unwrap(); // Safe because of required_unless_present
            let today = Utc::now().date_naive();
            let (start, end) = period_range(period.clone(), today);
            let filtered: Vec<Record> = records
                .into_iter()
                .filter(|x| x.date >= start && x.date <= end)
                .collect();

            if filtered.is_empty() {
                warn("no records in selected period");
                return;
            }

            let plugin_name = if let Some(p) = plugin {
                p.clone()
            } else {
                let plugins = discover_plugins();
                if plugins.is_empty() {
                    die("No plugins available. Use --list-plugins to see setup instructions.");
                } else if plugins.len() == 1 {
                    plugins[0].clone()
                } else {
                    die("Multiple plugins available, specify one with --plugin <name>");
                }
            };

            // Load plugin config
            let config_path = plugin_dir().join(format!("timelog-{plugin_name}.json"));
            let config = if config_path.exists() {
                let config_str = fs::read_to_string(config_path)
                    .unwrap_or_else(|_| die("Failed to read plugin config"));
                serde_json::from_str(&config_str)
                    .unwrap_or_else(|_| die("Invalid plugin config JSON"))
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            let period_str = format!("{period:?}").to_lowercase();
            let input = PluginInput {
                records: filtered,
                period: period_str,
                config,
            };

            info(&format!("Executing plugin: {}", emph(&plugin_name)));
            if *dry_run {
                info("(dry run mode)");
            }

            match execute_plugin(&plugin_name, &input, *dry_run) {
                Ok(output) => {
                    if output.success {
                        info(&output.message.to_string());
                        if let Some(count) = output.uploaded_count {
                            info(&format!("Processed {count} records"));
                        }
                        if !output.errors.is_empty() {
                            warn("Some warnings occurred:");
                            for error in output.errors {
                                warn(&format!("  {error}"));
                            }
                        }
                    } else {
                        warn(&format!("Plugin failed: {}", output.message));
                        for error in output.errors {
                            warn(&format!("  {error}"));
                        }
                    }
                }
                Err(e) => die(&format!("Plugin execution failed: {e}")),
            }
        }
    }
}
