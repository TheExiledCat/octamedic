use std::{
    backtrace::Backtrace,
    fs, io,
    path::{Path, PathBuf},
};

use crate::app::files;

const FILE_NAME: &str = "crash.log";

/// Where the last crash is written: the config folder, or the temp folder if there isn't one.
pub fn path() -> PathBuf {
    let folder = files::get_config_directory_root().unwrap_or_else(std::env::temp_dir);
    return folder.join(FILE_NAME);
}

/// Panics go in the crash log as well as wherever they went before, since a Windows GUI program
/// has no console to print them to.
pub fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let error = format!("{info}\n\n{}", Backtrace::force_capture());
        let _ = write(&path(), &report(&error, &[]));
    }));
}

/// The error that ended `main`, with the startup errors from before it.
pub fn log_error(error: &str, startup_errors: &[String]) {
    eprintln!("{error}");
    let _ = write(&path(), &report(error, startup_errors));
}

fn report(error: &str, startup_errors: &[String]) -> String {
    let mut report = format!(
        "OctaMEDIC {} crashed:\n{error}\n",
        env!("CARGO_PKG_VERSION")
    );
    if !startup_errors.is_empty() {
        report.push_str("\nBefore that:\n");
        for startup_error in startup_errors {
            report.push_str(&format!("{startup_error}\n"));
        }
    }
    return report;
}

// only the last crash is kept
fn write(path: &Path, report: &str) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    return fs::write(path, report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lists_the_startup_errors() {
        let report = report("No GPU", &["NO AUDIO: no device".to_owned()]);
        assert_eq!(
            report,
            format!(
                "OctaMEDIC {} crashed:\nNo GPU\n\nBefore that:\nNO AUDIO: no device\n",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn writes_over_the_last_crash() {
        let folder = std::env::temp_dir().join(format!("octamedic-crash-{}", std::process::id()));
        let path = folder.join(FILE_NAME);
        write(&path, "first").unwrap();
        write(&path, "second").unwrap();
        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(written, "second");
    }
}
