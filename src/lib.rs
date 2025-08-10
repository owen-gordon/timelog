use chrono::{DateTime, Datelike, Days, Local, NaiveDate, SecondsFormat, Utc, Weekday};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::IsTerminal;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub enum Period {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    YTD,
    LastYear,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Start {
        task: String,
        #[arg(short, long)]
        project: Option<String>,
    },
    Pause,
    Resume,
    Stop,
    Report {
        period: Period,
        #[arg(short, long)]
        project: Option<String>,
    },
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
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Record {
    pub task: String,
    pub duration_ms: i64,
    pub date: NaiveDate,
    pub project: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct State {
    pub timestamp: DateTime<Utc>,
    pub task: String,
    pub active: bool,
    pub project: Option<String>,
}

#[derive(Serialize)]
pub struct PluginInput {
    pub records: Vec<Record>,
    pub period: String,
    pub config: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct PluginOutput {
    pub success: bool,
    pub uploaded_count: Option<usize>,
    pub message: String,
    pub errors: Vec<String>,
}

pub fn record_path() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_RECORD_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog-record
    PathBuf::from(env::var("HOME").expect("$HOME not set")).join(".timelog-record")
}

pub fn state_path() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_STATE_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog-state
    PathBuf::from(env::var("HOME").expect("$HOME not set")).join(".timelog-state")
}

pub fn plugin_dir() -> PathBuf {
    // Check for custom path via environment variable first
    if let Ok(custom_path) = env::var("TIMELOG_PLUGIN_PATH") {
        return PathBuf::from(custom_path);
    }
    // Default to ~/.timelog/plugins
    PathBuf::from(env::var("HOME").expect("$HOME not set"))
        .join(".timelog")
        .join("plugins")
}

pub fn discover_plugins() -> Vec<String> {
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
            if path.is_file()
                && path.file_name()?.to_str()?.starts_with("timelog-")
                && !path.file_name()?.to_str()?.ends_with(".json")
            {
                // Check if file is executable
                use std::os::unix::fs::PermissionsExt;
                let metadata = path.metadata().ok()?;
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 != 0 {
                    // Check if any execute bit is set
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

pub fn execute_plugin(
    plugin_name: &str,
    input: &PluginInput,
    dry_run: bool,
) -> Result<PluginOutput, String> {
    let plugin_path = plugin_dir().join(format!("timelog-{plugin_name}"));

    if !plugin_path.exists() {
        return Err(format!(
            "Plugin '{plugin_name}' not found at {plugin_path:?}"
        ));
    }

    let mut cmd = Command::new(&plugin_path);
    if dry_run {
        cmd.arg("--dry-run");
    }

    let input_json =
        serde_json::to_string(input).map_err(|e| format!("Failed to serialize input: {e}"))?;

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start plugin: {e}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(input_json.as_bytes())
            .map_err(|e| format!("Failed to write to plugin stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for plugin: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Plugin failed with exit code {:?}: {}",
            output.status.code(),
            stderr
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse plugin output: {e}"))
}

pub fn is_tty() -> bool {
    use std::io::stdout;
    stdout().is_terminal()
}

pub fn emph(s: &str) -> String {
    // bold if TTY, plain otherwise
    if is_tty() {
        format!("\x1b[1m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn info(msg: &str) {
    println!("{msg}");
}

pub fn warn(msg: &str) {
    eprintln!("warning: {msg}");
}

pub fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

// ---------- duration pretty ----------
pub fn fmt_hms_ms(ms: i64) -> String {
    let total = ms / 1000;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let frac = ms % 1000;
    format!("{h:02}:{m:02}:{s:02}.{frac:03}")
}

pub fn clamp_nonneg(ms: i64) -> i64 {
    if ms < 0 { 0 } else { ms }
}

pub fn fmt_ts(dt: DateTime<Utc>) -> String {
    let local_dt: DateTime<Local> = dt.with_timezone(&Local);
    // ISO8601, no timezone ambiguity (UTC); change to .to_rfc3339() if you prefer
    local_dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn write<T>(t: T, file: File)
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let serialized = serde_json::to_string_pretty(&t).expect("Unable to serialize");

    let mut writer = BufWriter::new(file);

    writer
        .write_all(serialized.as_bytes())
        .expect("Unable to write");
    writer.flush().expect("Unable to flush");
}

pub fn read<T>(file: File) -> T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).expect("Unable to read")
}

pub fn period_range(period: Period, today: NaiveDate) -> (NaiveDate, NaiveDate) {
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

pub fn fmt_duration(ms: i64) -> String {
    let total_secs = ms / 1000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if s == 0 {
        format!("{h:02}h{m:02}m")
    } else {
        format!("{h:02}h{m:02}m{s:02}s")
    }
}

pub fn fmt_record_for_period(r: &Record, period: Period, _today: NaiveDate) -> String {
    // Tailor the date label based on the period
    let date_label = match period {
        Period::Today => "Today".to_string(),
        Period::Yesterday => "Yesterday".to_string(),
        Period::ThisWeek | Period::LastWeek => {
            // e.g. "Mon 08-04"
            format!(
                "{} {:02}-{:02}",
                weekday_short(r.date.weekday()),
                r.date.month(),
                r.date.day()
            )
        }
        Period::ThisMonth | Period::LastMonth => {
            // e.g. "08-04"
            format!("{:02}-{:02}", r.date.month(), r.date.day())
        }
        Period::YTD | Period::LastYear => r.date.to_string(), // YYYY-MM-DD
    };

    format!(
        "• {:<18} {}  ({})",
        r.task,
        fmt_duration(r.duration_ms),
        date_label
    )
}

pub fn weekday_short(w: Weekday) -> &'static str {
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

pub fn print_report(
    period: Period,
    start: NaiveDate,
    end: NaiveDate,
    rows: &[Record],
    project_filter: &Option<String>,
) {
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

    let title_suffix = match project_filter {
        Some(p) => format!(" for project {}", emph(p)),
        None => String::new(),
    };
    println!(
        "{}{} ({start}..{end})",
        emph(&format!("{title} report")),
        title_suffix
    );

    // column widths
    let mut task_w = "TASK".len();
    let mut project_w = "PROJECT".len();
    for r in rows {
        task_w = task_w.max(r.task.len());
        if let Some(p) = &r.project {
            project_w = project_w.max(p.len());
        }
    }

    let hdr_task = "TASK";
    let hdr_project = "PROJECT";
    let hdr_date = "DATE";
    let hdr_dur = "DURATION";

    println!("{hdr_task:<task_w$}  {hdr_project:<project_w$}  {hdr_date:<10}  {hdr_dur:>10}");
    println!(
        "{}  {}  {}  {}",
        "-".repeat(task_w),
        "-".repeat(project_w),
        "-".repeat(10),
        "-".repeat(10),
    );

    let mut total_ms: i64 = 0;
    for r in rows {
        total_ms += r.duration_ms;
        let project_str = r.project.as_deref().unwrap_or("-");
        println!(
            "{:<task_w$}  {:<project_w$}  {:<10}  {:>10}",
            r.task,
            project_str,
            r.date, // always ISO date for CLI clarity
            fmt_duration(r.duration_ms),
            task_w = task_w,
            project_w = project_w
        );
    }

    println!(
        "{}  {}  {}  {}",
        "-".repeat(task_w),
        "-".repeat(project_w),
        "-".repeat(10),
        "-".repeat(10),
    );
    println!(
        "{:<task_w$}  {:<project_w$}  {:<10}  {:>10}",
        "TOTAL",
        "",
        "",
        fmt_duration(total_ms),
        task_w = task_w,
        project_w = project_w
    );
}

pub fn load_records() -> Result<Vec<Record>, String> {
    let file = File::open(record_path()).map_err(|_| "no records found".to_string())?;
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file);

    let mut records: Vec<Record> = Vec::new();
    for result in rdr.records() {
        let record_result = result.map_err(|e| format!("Unable to read CSV record: {e}"))?;
        let record = if record_result.len() == 3 {
            // Old format without project
            Record {
                task: record_result[0].to_string(),
                duration_ms: record_result[1]
                    .parse()
                    .map_err(|_| "Invalid duration".to_string())?,
                date: record_result[2]
                    .parse()
                    .map_err(|_| "Invalid date".to_string())?,
                project: None,
            }
        } else if record_result.len() >= 4 {
            // New format with project
            let project = if record_result[3].is_empty() {
                None
            } else {
                Some(record_result[3].to_string())
            };
            Record {
                task: record_result[0].to_string(),
                duration_ms: record_result[1]
                    .parse()
                    .map_err(|_| "Invalid duration".to_string())?,
                date: record_result[2]
                    .parse()
                    .map_err(|_| "Invalid date".to_string())?,
                project,
            }
        } else {
            return Err("Invalid CSV record format".to_string());
        };
        records.push(record);
    }
    Ok(records)
}

pub fn save_record(record: &Record) -> Result<(), String> {
    let f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(record_path())
        .map_err(|e| format!("Failed to open record file: {e}"))?;
    let empty = f.metadata().map(|m| m.len() == 0).unwrap_or(true);
    let mut wtr = csv::WriterBuilder::new().has_headers(empty).from_writer(f);
    wtr.serialize(record)
        .map_err(|e| format!("Failed to write record: {e}"))?;
    wtr.flush()
        .map_err(|e| format!("Failed to flush record: {e}"))?;
    Ok(())
}

pub fn load_state() -> Result<State, String> {
    let file = File::open(state_path()).map_err(|_| "no state file found".to_string())?;
    Ok(read(file))
}

pub fn save_state(state: &State) -> Result<(), String> {
    let file =
        File::create(state_path()).map_err(|e| format!("Unable to create state file: {e}"))?;
    write(state.clone(), file);
    Ok(())
}

pub fn delete_state() -> Result<(), String> {
    fs::remove_file(state_path()).map_err(|e| format!("Unable to delete state file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_period_range_today() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let (start, end) = period_range(Period::Today, today);
        assert_eq!(start, today);
        assert_eq!(end, today);
    }

    #[test]
    fn test_period_range_yesterday() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
        let (start, end) = period_range(Period::Yesterday, today);
        assert_eq!(start, yesterday);
        assert_eq!(end, yesterday);
    }

    #[test]
    fn test_period_range_this_week() {
        // Monday, Jan 15, 2024
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let (start, end) = period_range(Period::ThisWeek, today);
        assert_eq!(start, today); // Should be Monday
        assert_eq!(end, today);

        // Wednesday, Jan 17, 2024
        let today = NaiveDate::from_ymd_opt(2024, 1, 17).unwrap();
        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let (start, end) = period_range(Period::ThisWeek, today);
        assert_eq!(start, monday);
        assert_eq!(end, today);
    }

    #[test]
    fn test_period_range_last_week() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday
        let (start, end) = period_range(Period::LastWeek, today);
        let expected_start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap(); // Previous Monday
        let expected_end = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap(); // Previous Sunday
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_period_range_this_month() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let (start, end) = period_range(Period::ThisMonth, today);
        let expected_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, today);
    }

    #[test]
    fn test_period_range_last_month() {
        let today = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let (start, end) = period_range(Period::LastMonth, today);
        let expected_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_period_range_last_month_january() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let (start, end) = period_range(Period::LastMonth, today);
        let expected_start = NaiveDate::from_ymd_opt(2023, 12, 1).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_period_range_ytd() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let (start, end) = period_range(Period::YTD, today);
        let expected_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, today);
    }

    #[test]
    fn test_period_range_last_year() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let (start, end) = period_range(Period::LastYear, today);
        let expected_start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_fmt_duration() {
        assert_eq!(fmt_duration(0), "00h00m");
        assert_eq!(fmt_duration(1000), "00h00m01s");
        assert_eq!(fmt_duration(60000), "00h01m");
        assert_eq!(fmt_duration(61000), "00h01m01s");
        assert_eq!(fmt_duration(3600000), "01h00m");
        assert_eq!(fmt_duration(3661000), "01h01m01s");
    }

    #[test]
    fn test_fmt_hms_ms() {
        assert_eq!(fmt_hms_ms(0), "00:00:00.000");
        assert_eq!(fmt_hms_ms(1500), "00:00:01.500");
        assert_eq!(fmt_hms_ms(61500), "00:01:01.500");
        assert_eq!(fmt_hms_ms(3661500), "01:01:01.500");
    }

    #[test]
    fn test_clamp_nonneg() {
        assert_eq!(clamp_nonneg(-100), 0);
        assert_eq!(clamp_nonneg(0), 0);
        assert_eq!(clamp_nonneg(100), 100);
    }

    #[test]
    fn test_weekday_short() {
        assert_eq!(weekday_short(Weekday::Mon), "Mon");
        assert_eq!(weekday_short(Weekday::Tue), "Tue");
        assert_eq!(weekday_short(Weekday::Wed), "Wed");
        assert_eq!(weekday_short(Weekday::Thu), "Thu");
        assert_eq!(weekday_short(Weekday::Fri), "Fri");
        assert_eq!(weekday_short(Weekday::Sat), "Sat");
        assert_eq!(weekday_short(Weekday::Sun), "Sun");
    }

    #[test]
    fn test_emph() {
        // The function will return either plain text or emphasized text
        // depending on TTY status. Both are valid results.
        let result = emph("test");
        assert!(result == "test" || result == "\x1b[1mtest\x1b[0m");
    }
}
