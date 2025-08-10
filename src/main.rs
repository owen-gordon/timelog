use clap::{Parser, Subcommand, ValueEnum};
use std;
use std::env;
use std::path::PathBuf;
use std::fs::{File, self, OpenOptions};
use std::io::{Write, BufWriter, BufReader};
use std::process::Command;
use serde::{Deserialize, Serialize};
use serde_json;
use chrono::{DateTime, Datelike, Days, NaiveDate, SecondsFormat, Utc, Weekday, Local};
use std::io::IsTerminal;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
#[clap(rename_all = "kebab_case")]
enum Period {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    YTD,
    LastYear
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start { task: String },
    Pause,
    Resume,
    Stop,
    Report { period: Period },
    Status,
    Upload { 
        #[arg(short, long)]
        plugin: Option<String>,
        #[arg(required_unless_present = "list_plugins")]
        period: Option<Period>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        list_plugins: bool,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Record {
    task: String,
    duration_ms: i64,
    date: NaiveDate
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    timestamp: DateTime<Utc>,  // if state is active: timestamp is when task started. if inactive: timestamp is when previous duration after epoch
    task: String,
    active: bool
}

#[derive(Serialize)]
struct PluginInput {
    records: Vec<Record>,
    period: String,
    config: serde_json::Value,
}

#[derive(Deserialize)]
struct PluginOutput {
    success: bool,
    uploaded_count: Option<usize>,
    message: String,
    errors: Vec<String>,
}

fn record_path() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_RECORD_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog-record
    return PathBuf::from(env::var("HOME").expect("$HOME not set")).join(".timelog-record");
}

fn state_path() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_STATE_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog-state
    return PathBuf::from(env::var("HOME").expect("$HOME not set")).join(".timelog-state");
}

fn plugin_dir() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_PLUGIN_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog/plugins
    PathBuf::from(env::var("HOME").expect("$HOME not set"))
        .join(".timelog")
        .join("plugins")
}

fn discover_plugins() -> Vec<String> {
    let plugin_path = plugin_dir();
    if !plugin_path.exists() {
        return Vec::new();
    }
    
    fs::read_dir(plugin_path)
        .unwrap_or_else(|_| die("Cannot read plugin directory"))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            // Only include executable files that start with "timelog-" and don't end with ".json"
            if path.is_file() && 
               path.file_name()?.to_str()?.starts_with("timelog-") &&
               !path.file_name()?.to_str()?.ends_with(".json") {
                // Check if file is executable
                use std::os::unix::fs::PermissionsExt;
                let metadata = path.metadata().ok()?;
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 != 0 { // Check if any execute bit is set
                    let stem = path.file_stem()?.to_str()?;
                    // Remove "timelog-" prefix for display
                    if stem.len() > 8 {
                        Some(stem[8..].to_string())
                    } else {
                        Some(stem.to_string())
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn execute_plugin(plugin_name: &str, input: &PluginInput, dry_run: bool) -> Result<PluginOutput, String> {
    let plugin_path = plugin_dir().join(format!("timelog-{}", plugin_name));
    
    if !plugin_path.exists() {
        return Err(format!("Plugin '{}' not found at {}", plugin_name, plugin_path.display()));
    }

    let mut cmd = Command::new(&plugin_path);
    if dry_run {
        cmd.arg("--dry-run");
    }

    let input_json = serde_json::to_string(input)
        .map_err(|e| format!("Failed to serialize input: {}", e))?;

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start plugin: {}", e))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input_json.as_bytes())
            .map_err(|e| format!("Failed to write to plugin stdin: {}", e))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to wait for plugin: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Plugin failed with exit code {:?}: {}", output.status.code(), stderr));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse plugin output: {}", e))
}

fn is_tty() -> bool {
    use std::io::stdout;
    stdout().is_terminal()
}

fn emph(s: &str) -> String {
    // bold if TTY, plain otherwise
    if is_tty() { format!("\x1b[1m{}\x1b[0m", s) } else { s.to_string() }
}

fn info(msg: &str) {
    println!("{}", msg);
}

fn warn(msg: &str) {
    eprintln!("warning: {}", msg);
}

fn die(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

// ---------- duration pretty ----------
fn fmt_hms_ms(ms: i64) -> String {
    let total = ms / 1000;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let frac = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, frac)
}

fn clamp_nonneg(ms: i64) -> i64 { if ms < 0 { 0 } else { ms } }

fn fmt_ts(dt: DateTime<Utc>) -> String {
    let local_dt: DateTime<Local> = dt.with_timezone(&Local);
    // ISO8601, no timezone ambiguity (UTC); change to .to_rfc3339() if you prefer
    local_dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn write<T>(t: T, file: File)
where
    T: Serialize + for<'de> Deserialize<'de>
{
    let serialized = serde_json::to_string_pretty(&t).expect("Unable to serialize");

    let mut writer = BufWriter::new(file);

    writer.write_all(serialized.as_bytes()).expect("Unable to write");
    writer.flush().expect("Unable to flush");
}

fn read<T>(file: File) -> T 
where
    T: Serialize + for<'de> Deserialize<'de>
{
    let reader = BufReader::new(file);

    return serde_json::from_reader(reader).expect("Unable to read");
}

fn period_range(period: Period, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    // inclusive [start, end]
    match period {
        Period::Today => (today, today),
        Period::Yesterday => {
            let y = today - Days::new(1);
            (y, y)
        }
        Period::ThisWeek => {
            let start = today - Days::new(today.weekday().num_days_from_monday() as u64);
            (start, today)
        }
        Period::LastWeek => {
            let this_week_start = today - Days::new(today.weekday().num_days_from_monday() as u64);
            let last_week_start = this_week_start - Days::new(7);
            let last_week_end = this_week_start - Days::new(1);
            (last_week_start, last_week_end)
        }
        Period::ThisMonth => {
            let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            (start, today)
        }
        Period::LastMonth => {
            let (y, m) = if today.month() == 1 {
                (today.year() - 1, 12)
            } else {
                (today.year(), today.month() - 1)
            };
            let start = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
            // first day of *this* month, minus one day
            let this_month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            let end = this_month_start - Days::new(1);
            (start, end)
        }
        Period::YTD => {
            let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            (start, today)
        }
        Period::LastYear => {
            let start = NaiveDate::from_ymd_opt(today.year() - 1, 1, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(today.year() - 1, 12, 31).unwrap();
            (start, end)
        }
    }
}

fn fmt_duration(ms: i64) -> String {
    let total_secs = ms / 1000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if s == 0 {
        format!("{:02}h{:02}m", h, m)
    } else {
        format!("{:02}h{:02}m{:02}s", h, m, s)
    }
}

fn fmt_record_for_period(r: &Record, period: Period, _today: NaiveDate) -> String {
    // Tailor the date label based on the period
    let date_label = match period {
        Period::Today => "Today".to_string(),
        Period::Yesterday => "Yesterday".to_string(),
        Period::ThisWeek | Period::LastWeek => {
            // e.g. "Mon 08-04"
            format!("{} {:02}-{:02}", weekday_short(r.date.weekday()), r.date.month(), r.date.day())
        }
        Period::ThisMonth | Period::LastMonth => {
            // e.g. "08-04"
            format!("{:02}-{:02}", r.date.month(), r.date.day())
        }
        Period::YTD | Period::LastYear => r.date.to_string(), // YYYY-MM-DD
    };

    format!("• {:<18} {}  ({})", r.task, fmt_duration(r.duration_ms), date_label)
}

fn weekday_short(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn print_report(period: Period, start: NaiveDate, end: NaiveDate, rows: &[Record]) {
    let title = match period {
        Period::Today => "Today",
        Period::Yesterday => "Yesterday",
        Period::ThisWeek => "This Week",
        Period::LastWeek => "Last Week",
        Period::ThisMonth => "This Month",
        Period::LastMonth => "Last Month",
        Period::YTD => "Year To Date",
        Period::LastYear => "Last Year",
    };

    println!("{} ({start}..{end})", emph(&format!("{} report", title)));

    // column widths
    let mut task_w = "TASK".len();
    for r in rows {
        task_w = task_w.max(r.task.len());
    }

    let hdr_task = "TASK";
    let hdr_date = "DATE";
    let hdr_dur  = "DURATION";

    println!("{:<task_w$}  {:<10}  {:>10}",
             hdr_task, hdr_date, hdr_dur, task_w = task_w);
    println!(
        "{}  {}  {}",
        "-".repeat(task_w),
        "-".repeat(10),
        "-".repeat(10),
    );

    let mut total_ms: i64 = 0;
    for r in rows {
        total_ms += r.duration_ms;
        println!(
            "{:<task_w$}  {:<10}  {:>10}",
            r.task,
            r.date,                   // always ISO date for CLI clarity
            fmt_duration(r.duration_ms),
            task_w = task_w
        );
    }

    println!(
        "{}  {}  {}",
        "-".repeat(task_w),
        "-".repeat(10),
        "-".repeat(10),
    );
    println!(
        "{:<task_w$}  {:<10}  {:>10}",
        "TOTAL",
        "",
        fmt_duration(total_ms),
        task_w = task_w
    );
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { task } => {
            if state_path().exists() {
                die("a task is already in progress; run `timelog pause` or `timelog stop`");
            }

            let file = File::create(state_path()).expect("Unable to create state file");
            let state = State{ timestamp: Utc::now(), task: task.to_string(), active: true };
            write(state, file);

            info(&format!("started {}", emph(task)));
        }

        Commands::Pause => {
            if !state_path().exists() {
                die("no active task to pause");
            }

            let file = File::open(state_path()).expect("Unable to open state");
            let state: State = read(file);
            if !state.active {
                die("task is already paused; use `timelog resume`");
            }

            let elapsed = Utc::now() - state.timestamp;
            let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
            let timestamp = epoch + elapsed;

            let paused_state = State{ timestamp, task: state.task.clone(), active: false };
            let pause_file = File::create(state_path()).expect("Unable to create state file");
            write(paused_state, pause_file);

            info(&format!("paused {}  (elapsed {})",
                emph(&state.task),
                fmt_hms_ms(elapsed.num_milliseconds()),
            ));
        }

        Commands::Resume => {
            if !state_path().exists() {
                die("no paused task to resume");
            }

            let file = File::open(state_path()).expect("Unable to open state");
            let state: State = read(file);
            if state.active {
                die("task is already running");
            }

            let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
            let elapsed = state.timestamp - epoch;
            let timestamp = Utc::now() - elapsed;

            let active_state = State{ timestamp, task: state.task.clone(), active: true };
            let resume_file = File::create(state_path()).expect("Unable to create state file");
            write(active_state, resume_file);

            info(&format!("resumed {}", emph(&state.task)));
        }

        Commands::Stop => {
            if !state_path().exists() {
                die("no task to stop");
            }

            let file = File::open(state_path()).expect("Unable to open state");
            let state: State = read(file);

            let elapsed = if state.active {
                Utc::now() - state.timestamp
            } else {
                let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
                state.timestamp - epoch
            };

            let record = Record {
                task: state.task.clone(),
                duration_ms: elapsed.num_milliseconds(),
                date: Utc::now().naive_local().date(),
            };

            let f = OpenOptions::new().create(true).write(true).append(true)
                .open(record_path()).expect("open record file for append");
            let empty = f.metadata().map(|m| m.len() == 0).unwrap_or(true);
            let mut wtr = csv::WriterBuilder::new().has_headers(empty).from_writer(f);
            wtr.serialize(&record).expect("write record");
            wtr.flush().expect("flush record");

            fs::remove_file(state_path()).expect("Unable to delete state file");

            info(&format!(
                "recorded {}  {} on {}",
                emph(&record.task),
                fmt_hms_ms(record.duration_ms),
                record.date,
            ));
        }

        Commands::Report { period } => {
            let file = File::open(record_path()).unwrap_or_else(|_| die("no records found"));
            let mut rdr = csv::Reader::from_reader(file);

            let mut records: Vec<Record> = Vec::new();
            for result in rdr.deserialize() {
                let record: Record = result.expect("Unable to deserialize record");
                records.push(record);
            }

            let today = Utc::now().date_naive();
            let (start, end) = period_range(period.clone(), today);

            let mut filtered: Vec<Record> = records
                .into_iter()
                .filter(|x| x.date >= start && x.date <= end)
                .collect();

            if filtered.is_empty() {
                warn("no records in selected period");
                return;
            }

            // sort by date, then task
            filtered.sort_by_key(|r| (r.date, r.task.clone()));

            print_report(period.clone(), start, end, &filtered);
        }

        Commands::Status => {
            if !state_path().exists() {
                die("no task to provide status");
            }

            let file = File::open(state_path()).expect("Unable to open state");
            let state: State = read(file);

            let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

            // If active, elapsed = now - started_at; if paused, elapsed = stored
            let (elapsed_ms, since_ts, status_str) = if state.active {
                let e = (Utc::now() - state.timestamp).num_milliseconds();
                (clamp_nonneg(e), state.timestamp, "active")
            } else {
                let e = (state.timestamp - epoch).num_milliseconds();
                (clamp_nonneg(e), state.timestamp, "paused")
            };

            // Pretty, concise status lines
            if state.active {
                // e.g., "active 00:42:10.123 since 2025-08-08T17:20:11Z  —  task: compile"
                info(&format!(
                    "{}  {}  since {}  —  task: {}",
                    emph("active"),
                    fmt_hms_ms(elapsed_ms),
                    fmt_ts(since_ts),
                    emph(&state.task),
                ));
            } else {
                // When paused, `since_ts` is the pause timestamp encoded in state.timestamp
                info(&format!(
                    "{}  accumulated {}  —  task: {}",
                    emph("paused"),
                    fmt_hms_ms(elapsed_ms),
                    emph(&state.task),
                ));
            }
        }

        Commands::Upload { plugin, period, dry_run, list_plugins } => {
            if *list_plugins {
                let plugins = discover_plugins();
                if plugins.is_empty() {
                    info("No plugins found");
                    info(&format!("Place plugin scripts in: {}", plugin_dir().display()));
                    info("Plugin scripts should be named 'timelog-<name>' and be executable");
                } else {
                    info("Available plugins:");
                    for p in plugins {
                        println!("  • {}", p);
                    }
                }
                return;
            }

            // Load records for the specified period
            let file = File::open(record_path()).unwrap_or_else(|_| die("no records found"));
            let mut rdr = csv::Reader::from_reader(file);
            let mut records: Vec<Record> = Vec::new();
            for result in rdr.deserialize() {
                let record: Record = result.expect("Unable to deserialize record");
                records.push(record);
            }

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
            let config_path = plugin_dir().join(format!("timelog-{}.json", plugin_name));
            let config = if config_path.exists() {
                let config_str = fs::read_to_string(config_path)
                    .unwrap_or_else(|_| die("Failed to read plugin config"));
                serde_json::from_str(&config_str)
                    .unwrap_or_else(|_| die("Invalid plugin config JSON"))
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            let period_str = format!("{:?}", period).to_lowercase();
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
                        info(&format!("{}", output.message));
                        if let Some(count) = output.uploaded_count {
                            info(&format!("Processed {} records", count));
                        }
                        if !output.errors.is_empty() {
                            warn("Some warnings occurred:");
                            for error in output.errors {
                                warn(&format!("  {}", error));
                            }
                        }
                    } else {
                        warn(&format!("Plugin failed: {}", output.message));
                        for error in output.errors {
                            warn(&format!("  {}", error));
                        }
                    }
                }
                Err(e) => die(&format!("Plugin execution failed: {}", e)),
            }
        }
    }
}
