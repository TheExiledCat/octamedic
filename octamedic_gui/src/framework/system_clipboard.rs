use rustuition::clipboard::{self, TextClipboard};

struct System(arboard::Clipboard);

impl TextClipboard for System {
    fn text(&mut self) -> Option<String> {
        return self.0.get_text().ok();
    }

    fn set_text(&mut self, text: &str) {
        let _ = self.0.set_text(text);
    }
}

/// Points the text boxes' copy and paste at the system clipboard. Without one, like a Linux
/// session with no display, they copy inside the app only.
pub fn install() -> Result<(), String> {
    let system =
        arboard::Clipboard::new().map_err(|error| format!("No system clipboard: {error}"))?;
    clipboard::install(Box::new(System(system)));
    return Ok(());
}
