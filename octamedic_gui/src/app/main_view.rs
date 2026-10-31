use ggez::input::keyboard::KeyCode;

use crate::{
    app::{
        copyright_bar::CopyrightBar, debug_console::DebugConsole, footer::Footer, header::Header,
        main::MainScreens, mode::MainMode, state::AppState, title_bar::TitleBar,
    },
    framework::{
        event::{Event, MouseButton},
        keymap::{Action, Key, KeyActions, Keymap},
        painter::{Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
    },
};

// heights in screen lines, top to bottom; the footer takes the rest of the screen
const TITLE_HEIGHT: i32 = 13;
const HEADER_HEIGHT: i32 = 64;
const EDITOR_HEIGHT: i32 = 120;

/// The debug console covers the top half of the screen.
const CONSOLE_HEIGHT: i32 = 128;

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
    screens: MainScreens,
    footer: Footer,
    console: DebugConsole,
    copyright: CopyrightBar,
}

impl MainView {
    pub fn new() -> Self {
        return Self {
            keys: Keymap::new("main").bind(Key::ctrl(KeyCode::Q), MainAction::Quit, "Quit"),
            title: TitleBar::new(),
            header: Header::new(),
            screens: MainScreens::new(),
            footer: Footer,
            console: DebugConsole::default(),
            copyright: CopyrightBar::default(),
        };
    }

    fn widgets(&mut self, mode: MainMode) -> Vec<&mut dyn OctamedicWidget> {
        let mut widgets: Vec<&mut dyn OctamedicWidget> = vec![&mut self.title, &mut self.header];
        widgets.extend(self.screens.screen(mode));
        widgets.push(&mut self.footer);
        return widgets;
    }
}

impl OctamedicWidget for MainView {
    fn update(&mut self, state: &mut AppState) {
        for widget in self.widgets(state.main_mode()) {
            widget.update(state);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // global keys are checked first so no child can swallow them
        let taken = self.console.handle_event(state, event)
            || self.copyright.handle_event(state, event)
            || self.dispatch(state, event)
            || self
                .widgets(state.main_mode())
                .into_iter()
                .any(|widget| widget.handle_event(state, event));
        let right_click = matches!(
            event,
            Event::MouseDown {
                button: MouseButton::Right,
                ..
            }
        );
        if !taken && right_click {
            self.copyright.toggle();
            return true;
        }
        return taken;
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        let (title, rest) = area.split_top(TITLE_HEIGHT);
        let (header, rest) = rest.split_top(HEADER_HEIGHT);
        let (editor, footer) = rest.split_top(EDITOR_HEIGHT);
        self.title.render(state, title, painter);
        self.copyright.render(state, title, painter);
        self.header.render(state, header, painter);
        match self.screens.screen(state.main_mode()) {
            Some(screen) => screen.render(state, editor, painter),
            None => painter.fill(editor, Pen::BLACK),
        }
        self.footer.render(state, footer, painter);
        let (console, _) = area.split_top(CONSOLE_HEIGHT);
        self.console.render(state, console, painter);
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

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn right_click(view: &mut MainView, state: &mut AppState) -> bool {
        let event = Event::MouseDown {
            pos: [300, 150],
            button: MouseButton::Right,
        };
        return view.handle_event(state, &event);
    }

    #[test]
    fn a_right_click_nothing_takes_shows_and_hides_the_copyright() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        assert!(right_click(&mut view, &mut state));
        assert!(view.copyright.is_visible());
        assert!(right_click(&mut view, &mut state));
        assert!(!view.copyright.is_visible());
    }
}
