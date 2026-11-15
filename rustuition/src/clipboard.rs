use std::cell::RefCell;

/// Where copied text goes and comes from, like the system clipboard. An app installs one with
/// [`install`], and text boxes use it for their copy, cut and paste keys.
pub trait TextClipboard {
    fn text(&mut self) -> Option<String>;

    fn set_text(&mut self, text: &str);
}

thread_local! {
    static INSTALLED: RefCell<Option<Box<dyn TextClipboard>>> = const { RefCell::new(None) };
    // what's used with nothing installed, so copying still works inside the app
    static INTERNAL: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Replaces the clipboard for this thread, which is the one the gadgets run on.
pub fn install(clipboard: Box<dyn TextClipboard>) {
    INSTALLED.with(|installed| *installed.borrow_mut() = Some(clipboard));
}

pub fn text() -> Option<String> {
    let text = INSTALLED.with(|installed| installed.borrow_mut().as_mut()?.text());
    return text.or_else(|| INTERNAL.with(|internal| Some(internal.borrow().clone())));
}

pub fn set_text(text: &str) {
    let installed = INSTALLED.with(|installed| {
        installed
            .borrow_mut()
            .as_mut()
            .map(|clipboard| clipboard.set_text(text))
    });
    if installed.is_none() {
        INTERNAL.with(|internal| *internal.borrow_mut() = text.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    #[test]
    fn keeps_text_inside_without_a_clipboard() {
        set_text("hello");
        assert_eq!(text().as_deref(), Some("hello"));
    }

    struct Shared(Rc<Cell<Option<&'static str>>>);

    impl TextClipboard for Shared {
        fn text(&mut self) -> Option<String> {
            return self.0.get().map(str::to_owned);
        }

        fn set_text(&mut self, _: &str) {
            self.0.set(Some("set"));
        }
    }

    #[test]
    fn installed_clipboard_takes_over() {
        let cell = Rc::new(Cell::new(None));
        install(Box::new(Shared(cell.clone())));
        set_text("ignored");
        assert_eq!(cell.get(), Some("set"));
        assert_eq!(text().as_deref(), Some("set"));
    }
}
