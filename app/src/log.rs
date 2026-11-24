//! A small log file, so problems can be reported: panics with backtraces,
//! GTK/GStreamer warnings, and the app's own notes (downloads, playback).
//! Kept at ~/.local/state/bible-app/bible-app.log and trimmed at startup.

use std::io::Write;
use std::path::PathBuf;

pub fn dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("bible-app")
}

pub fn path() -> PathBuf {
    dir().join("bible-app.log")
}

/// Append a line with a timestamp; also echoed to stderr.
pub fn write(level: &str, msg: &str) {
    let line = format!("{} {level:<7} {msg}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    eprint!("{line}");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path()) {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn info(msg: &str) {
    write("INFO", msg);
}

pub fn error(msg: &str) {
    write("ERROR", msg);
}

/// Start logging: trim the old log, catch panics and GLib warnings.
pub fn init() {
    let _ = std::fs::create_dir_all(dir());
    // keep the last ~200 KB
    if let Ok(text) = std::fs::read_to_string(path()) {
        if text.len() > 400_000 {
            let cut = text.len() - 200_000;
            let start = text[cut..].find('\n').map(|i| cut + i + 1).unwrap_or(cut);
            let _ = std::fs::write(path(), &text[start..]);
        }
    }
    info(&format!("Bible {} starting", env!("CARGO_PKG_VERSION")));
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        let bt = std::backtrace::Backtrace::force_capture();
        write("PANIC", &format!("{p}\n{bt}"));
        default(p);
    }));
    gtk::glib::log_set_default_handler(|domain, level, msg| {
        use gtk::glib::LogLevel;
        let lv = match level {
            LogLevel::Error => "ERROR",
            LogLevel::Critical => "CRIT",
            LogLevel::Warning => "WARN",
            LogLevel::Message => "MSG",
            _ => return,
        };
        write(lv, &format!("[{}] {msg}", domain.unwrap_or("")));
    });
}
