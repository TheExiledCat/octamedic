use std::{
    any::Any,
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::{alert, app::files};

const FILE_NAME: &str = "crash.log";
const MOST_NOTES: usize = 50;

// what goes at the top of the crash log if the app crashes later: the backend asked for, the
// adapter wgpu picked, startup errors and warnings
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());
// the first error of a run starts a new crash log, the rest are added to it
static STARTED: AtomicBool = AtomicBool::new(false);

/// Where the last crash is written: the config folder, or the temp folder if there isn't one.
pub fn path() -> PathBuf {
    let folder = files::get_config_directory_root().unwrap_or_else(std::env::temp_dir);
    return folder.join(FILE_NAME);
}

pub fn note(line: String) {
    let mut notes = NOTES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if notes.len() < MOST_NOTES {
        notes.push(line);
    }
}

/// Panics go in the crash log as well as wherever they went before, since a Windows GUI program
/// has no console to print them to.
pub fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        add(&format!("{info}\n\n{}", Backtrace::force_capture()));
        // a panic on another thread, like the engine's, doesn't close the window
        if thread::current().name() == Some("main") {
            alert::crashed(panic_message(info.payload()));
        }
    }));
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return message;
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message;
    }
    return "it panicked";
}

/// ggez ends its event loop on a graphics error with only an `error!` and an `eprintln!`, so
/// logging is the one place to catch it.
pub fn log_records() {
    if log::set_logger(&CrashLogger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

/// The error that ended `main`.
pub fn log_error(error: &str) {
    eprintln!("{error}");
    add(error);
    alert::crashed(error);
}

struct CrashLogger;

impl Log for CrashLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        return match metadata.level() {
            Level::Error | Level::Warn => true,
            // wgpu logs the adapter it picks, and nothing else, here
            Level::Info => metadata.target() == "wgpu_core::instance",
            Level::Debug | Level::Trace => false,
        };
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("{}: {}", record.target(), record.args());
        if record.level() != Level::Error {
            note(line);
            return;
        }
        add(&line);
        if record.target().starts_with("ggez") {
            alert::crashed(&record.args().to_string());
        }
    }

    fn flush(&self) {}
}

fn add(entry: &str) {
    let first = !STARTED.swap(true, Ordering::SeqCst);
    let _ = add_to(&path(), entry, first);
}

fn add_to(path: &Path, entry: &str, first: bool) -> io::Result<()> {
    if !first {
        let mut file = OpenOptions::new().append(true).create(true).open(path)?;
        return write!(file, "\n{entry}\n");
    }
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let notes = NOTES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    return fs::write(path, report(entry, &notes));
}

fn report(error: &str, notes: &[String]) -> String {
    let mut report = format!(
        "OctaMEDIC {} on {} {}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for line in notes {
        report.push_str(&format!("{line}\n"));
    }
    report.push_str(&format!("\n{error}\n"));
    return report;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_crash_log(name: &str) -> (PathBuf, PathBuf) {
        let folder = std::env::temp_dir().join(format!("octamedic-{name}-{}", std::process::id()));
        return (folder.clone(), folder.join(FILE_NAME));
    }

    #[test]
    fn report_puts_the_notes_before_the_error() {
        let notes = [
            "Graphics backend asked for: dx12".to_owned(),
            "NO AUDIO: no device".to_owned(),
        ];
        assert_eq!(
            report("No GPU", &notes),
            format!(
                "OctaMEDIC {} on {} {}\nGraphics backend asked for: dx12\nNO AUDIO: no device\n\nNo GPU\n",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        );
    }

    #[test]
    fn a_new_run_writes_over_the_last_crash() {
        let (folder, path) = temp_crash_log("crash-new-run");
        add_to(&path, "first", true).unwrap();
        add_to(&path, "second", true).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&folder);
        assert!(written.ends_with("\nsecond\n"));
        assert!(!written.contains("first"));
    }

    #[test]
    fn later_errors_in_a_run_are_added() {
        let (folder, path) = temp_crash_log("crash-same-run");
        add_to(&path, "begin_frame failed", true).unwrap();
        add_to(&path, "end_frame failed", false).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&folder);
        assert!(written.ends_with("\nbegin_frame failed\n\nend_frame failed\n"));
    }

    #[test]
    fn keeps_errors_warnings_and_the_picked_adapter() {
        let kept = |level, target| {
            let metadata = Metadata::builder().level(level).target(target).build();
            return CrashLogger.enabled(&metadata);
        };
        assert!(kept(Level::Error, "ggez::event"));
        assert!(kept(Level::Warn, "ggez::graphics::context"));
        assert!(kept(Level::Info, "wgpu_core::instance"));
        assert!(!kept(Level::Info, "wgpu_core::device"));
        assert!(!kept(Level::Debug, "wgpu_core::instance"));
    }
}
