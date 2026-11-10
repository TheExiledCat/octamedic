use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::crash_log;

const TITLE: &str = "OctaMEDIC crashed";
#[cfg(unix)]
const OPEN_BUTTON: &str = "Open crash log";
#[cfg(unix)]
const CLOSE_BUTTON: &str = "Close";

static SHOWN: AtomicBool = AtomicBool::new(false);

/// Says the app crashed, with a button that opens the crash log. Only the first crash of a run
/// gets one, since the errors after it come from the same thing.
pub fn crashed(error: &str) {
    if SHOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    let path = crash_log::path();
    let message = format!("{error}\n\nThe details are in {}.", path.display());
    if asks_to_open(&message) {
        open(&path);
    }
}

#[cfg(windows)]
fn asks_to_open(message: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_ICONERROR, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW,
    };

    let text = wide(&format!("{message}\n\nOpen the crash log?"));
    let title = wide(TITLE);
    let style = MB_YESNO | MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND;
    let answer = unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), style) };
    return answer == IDYES;
}

#[cfg(windows)]
fn open(path: &Path) {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    let operation = wide("open");
    let file = wide(&path.to_string_lossy());
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    return text.encode_utf16().chain(std::iter::once(0)).collect();
}

#[cfg(target_os = "macos")]
fn asks_to_open(message: &str) -> bool {
    let script = format!(
        "display alert {} message {} as critical buttons {{{}, {}}} default button {}",
        applescript_text(TITLE),
        applescript_text(message),
        applescript_text(CLOSE_BUTTON),
        applescript_text(OPEN_BUTTON),
        applescript_text(OPEN_BUTTON),
    );
    let Ok(output) = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
    else {
        return false;
    };
    return String::from_utf8_lossy(&output.stdout).contains(OPEN_BUTTON);
}

#[cfg(target_os = "macos")]
fn applescript_text(text: &str) -> String {
    return format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
}

#[cfg(target_os = "macos")]
fn open(path: &Path) {
    let _ = std::process::Command::new("open").arg(path).status();
}

// zenity on GNOME and most others, kdialog on KDE; with neither there's only the crash log
#[cfg(all(unix, not(target_os = "macos")))]
fn asks_to_open(message: &str) -> bool {
    use std::process::Command;

    let zenity = Command::new("zenity")
        .args([
            "--question",
            "--no-markup",
            "--title",
            TITLE,
            "--text",
            message,
        ])
        .args(["--ok-label", OPEN_BUTTON, "--cancel-label", CLOSE_BUTTON])
        .status();
    if let Ok(answer) = zenity {
        return answer.success();
    }
    let kdialog = Command::new("kdialog")
        .args(["--title", TITLE, "--warningyesno", message])
        .args(["--yes-label", OPEN_BUTTON, "--no-label", CLOSE_BUTTON])
        .status();
    return kdialog.is_ok_and(|answer| answer.success());
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[cfg(not(any(windows, unix)))]
fn asks_to_open(_message: &str) -> bool {
    return false;
}

#[cfg(not(any(windows, unix)))]
fn open(_path: &Path) {}
