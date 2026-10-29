use ggez::input::keyboard::KeyCode;

use crate::{
    app::{
        editor::BlockEditor, footer::Footer, header::Header, state::AppState, title_bar::TitleBar,
    },
    framework::{
        event::Event,
        keymap::{Action, Key, KeyActions, Keymap},
        painter::Painter,
        rect::Rect,
        widget::OctamedicWidget,
    },
};

// heights in screen lines, top to bottom; they add up to the screen's 256
const TITLE_HEIGHT: i32 = 13;
const HEADER_HEIGHT: i32 = 64;
const EDITOR_HEIGHT: i32 = 120;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MainAction {
    Quit,
}

impl Action for MainAction {}

/// The whole screen.
pub struct MainView {
    keys: Keymap<MainAction>,
    title: TitleBar,
    header: Header,
    editor: BlockEditor,
    footer: Footer,
}

impl MainView {
    pub fn new() -> Self {
        return Self {
            keys: Keymap::new("main").bind(Key::ctrl(KeyCode::Q), MainAction::Quit, "Quit"),
            title: TitleBar::new(),
            header: Header::new(),
            editor: BlockEditor::new(),
            footer: Footer,
        };
    }

    fn widgets(&mut self) -> [&mut dyn OctamedicWidget; 4] {
        return [
            &mut self.title,
            &mut self.header,
            &mut self.editor,
            &mut self.footer,
        ];
    }
}

impl OctamedicWidget for MainView {
    fn update(&mut self, state: &mut AppState) {
        for widget in self.widgets() {
            widget.update(state);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // global keys are checked first so no child can swallow them
        return self.dispatch(state, event)
            || self
                .widgets()
                .into_iter()
                .any(|widget| widget.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        let (title, rest) = area.split_top(TITLE_HEIGHT);
        let (header, rest) = rest.split_top(HEADER_HEIGHT);
        let (editor, footer) = rest.split_top(EDITOR_HEIGHT);
        self.title.render(state, title, painter);
        self.header.render(state, header, painter);
        self.editor.render(state, editor, painter);
        self.footer.render(state, footer, painter);
    }
}

impl KeyActions for MainView {
    type Action = MainAction;

    fn keymap(&self) -> &Keymap<MainAction> {
        return &self.keys;
    }

    fn perform(&mut self, action: MainAction, state: &mut AppState) {
        match action {
            MainAction::Quit => state.request_exit(),
        }
    }

    fn describe(action: MainAction) -> &'static str {
        return match action {
            MainAction::Quit => "Quit OctaMEDIC",
        };
    }
}
