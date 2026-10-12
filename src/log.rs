//! Tiny append-only logger.
//!
//! Writes sparse startup and error messages to `<dll dir>/<log_file>`.
//! Successful gameplay callbacks do not log. Debugger mirroring is disabled.

#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Level {
    pub fn parse(s: &str) -> Level {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "silent" => Level::Off,
            "error" | "err" => Level::Error,
            "warn" | "warning" => Level::Warn,
            "info" => Level::Info,
            "debug" => Level::Debug,
            "trace" | "verbose" => Level::Trace,
            _ => Level::Info,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Level::Off => "OFF",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOG_NAME: OnceLock<String> = OnceLock::new();
static MAX_BYTES: AtomicUsize = AtomicUsize::new(2 * 1024 * 1024);

/// Point the logger at `<dir>/<file>` and set the initial verbosity.
pub fn init(dir: &Path, file: &str, level: Level) {
    let _ = LOG_DIR.set(dir.to_path_buf());
    let _ = LOG_NAME.set(file.to_string());
    LEVEL.store(level as u8, Ordering::Relaxed);
    rotate_if_needed();
}

pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn enabled(level: Level) -> bool {
    (level as u8) <= LEVEL.load(Ordering::Relaxed)
}

pub fn path() -> Option<PathBuf> {
    Some(LOG_DIR.get()?.join(LOG_NAME.get()?))
}

fn rotate_if_needed() {
    if let Some(p) = path() {
        if let Ok(md) = std::fs::metadata(&p) {
            if md.len() as usize > MAX_BYTES.load(Ordering::Relaxed) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

pub fn log(level: Level, msg: &str) {
    if !enabled(level) {
        return;
    }
    let line = format!("[{}] [{}] {}\r\n", timestamp(), level.tag(), msg);
    if let Some(p) = path() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// Logging macros.  `log` is also the name of the `log` crate, so these carry a
/// `nlog_` prefix at the crate root and are usually imported under a short
/// alias, e.g. `use crate::{nlog_error as error};`.
///
/// Every macro checks the level *before* formatting, so a `trace!` on a hot
/// game path costs nothing when tracing is off.
#[macro_export]
macro_rules! nlog_error {
    ($($arg:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Error) {
            $crate::log::log($crate::log::Level::Error, &format!($($arg)*))
        }
    };
}
#[macro_export]
macro_rules! nlog_warn {
    ($($arg:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Warn) {
            $crate::log::log($crate::log::Level::Warn, &format!($($arg)*))
        }
    };
}
#[macro_export]
macro_rules! nlog_info {
    ($($arg:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Info) {
            $crate::log::log($crate::log::Level::Info, &format!($($arg)*))
        }
    };
}
#[macro_export]
macro_rules! nlog_debug {
    ($($arg:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Debug) {
            $crate::log::log($crate::log::Level::Debug, &format!($($arg)*))
        }
    };
}
#[macro_export]
macro_rules! nlog_trace {
    ($($arg:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Trace) {
            $crate::log::log($crate::log::Level::Trace, &format!($($arg)*))
        }
    };
}

/// UTC `YYYY-MM-DD hh:mm:ss.mmm` without pulling in a date crate.
pub fn timestamp() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs() as i64;
    let millis = d.subsec_millis();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, dd) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        y,
        m,
        dd,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        millis
    )
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}
