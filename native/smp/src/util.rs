//! What the bundle took from the game's own crates: a log, the clock and the profiler's spans.

/// The server's log is its standard error: a line each, tagged by the SMP's own `[smp]` prefix.
pub mod log {
    macro_rules! error {
        ($($t:tt)*) => { eprintln!("[ERROR] {}", format_args!($($t)*)) };
    }
    macro_rules! warn_ {
        ($($t:tt)*) => { eprintln!("[WARN] {}", format_args!($($t)*)) };
    }
    macro_rules! info {
        ($($t:tt)*) => { eprintln!("[INFO] {}", format_args!($($t)*)) };
    }
    pub(crate) use {error, info, warn_ as warn};
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// The game's profiler spans: the SMP's work isn't measured by them here.
pub mod spans {
    pub struct Span;
    pub fn guard(_name: &str) -> Span {
        Span
    }
}

/// The UTC date, `YYYY-MM-DD`, for the day's reward.
pub fn today() -> String {
    let (y, m, d) = civil(now_ms().div_euclid(86_400_000));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DDTHH:MM:SSZ`, for a log line.
pub fn stamp() -> String {
    let ms = now_ms();
    let (y, m, d) = civil(ms.div_euclid(86_400_000));
    let secs = ms.div_euclid(1000).rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// The civil date of a day count since 1970-01-01 (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}
