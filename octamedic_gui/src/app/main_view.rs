use ggez::input::keyboard::KeyCode;
use rustuition::{
    gadget::Gadget,
    painter::{Align, CHAR_HEIGHT, Painter, Pen, text_width},
    rect::Rect,
    text::Line,
};

use crate::{
    app::{
        copyright_bar::CopyrightBar,
        debug_console::DebugConsole,
        footer::{Footer, INFO_HEIGHT},
        header::Header,
        main::MainScreens,
        mode::MainMode,
        state::AppState,
        title_bar::TitleBar,
    },
    framework::{
        event::{Event, MouseButton},
        keymap::{Action, Key, KeyActions, Keymap},
    },
};

// the footer takes the rest of the screen
const TITLE_HEIGHT: i32 = 13;
const HEADER_HEIGHT: i32 = 64;
const EDITOR_HEIGHT: i32 = 120;

const CONSOLE_HEIGHT: i32 = 128;

const NOTIFICATION_PADDING: [i32; 2] = [2, 1];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MainAction {
    Quit,
    ToggleVideo,
}

impl Action for MainAction {}

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
            keys: Keymap::new("main")
                .bind(Key::ctrl(KeyCode::Q), MainAction::Quit, "Quit")
                // temporary, until there's a setting for it
                .bind(Key::ctrl(KeyCode::F12), MainAction::ToggleVideo, "PAL/NTSC"),
            title: TitleBar::new(),
            header: Header::new(),
            screens: MainScreens::new(),
            footer: Footer::new(),
            console: DebugConsole::default(),
            copyright: CopyrightBar::default(),
        };
    }

    fn widgets(&mut self, mode: MainMode) -> Vec<&mut dyn Gadget<AppState, Event>> {
        // the footer first, its song name box needs every click to know when it's left
        let mut widgets: Vec<&mut dyn Gadget<AppState, Event>> =
            vec![&mut self.footer, &mut self.title, &mut self.header];
        widgets.extend(self.screens.screen(mode));
        return widgets;
    }
}

impl Gadget<AppState, Event> for MainView {
    fn update(&mut self, state: &mut AppState) {
        for widget in self.widgets(state.main_mode()) {
            widget.update(state);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // global keys first, or a child eats them
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
            state.displays_off = !state.displays_off;
            return true;
        }
        return taken;
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        let (title, rest) = area.split_top(TITLE_HEIGHT);
        let (header, rest) = rest.split_top(HEADER_HEIGHT);
        // NTSC is too short for all of it, the editor gives up lines so the info line stays
        let editor_height = EDITOR_HEIGHT.min(rest.h - INFO_HEIGHT);
        let (editor, footer) = rest.split_top(editor_height);
        self.title.render(state, title, painter);
        self.copyright.render(state, title, painter);
        self.header.render(state, header, painter);
        match self.screens.screen(state.main_mode()) {
            Some(screen) => screen.render(state, editor, painter),
            None => {
                painter.fill(editor, Pen::BACKGROUND);

                painter.line_in(
                    editor,
                    &Line::from("Not implemented"),
                    Pen::SHINE,
                    Align::Center,
                );
            }
        }
        self.footer.render(state, footer, painter);
        let (console, _) = area.split_top(CONSOLE_HEIGHT);
        self.console.render(state, console, painter);
        if let Some(notification) = &state.notification {
            let [x, y] = NOTIFICATION_PADDING;
            let width = text_width(notification) + 2 * x;
            let box_area = Rect::new(area.x, area.y, width, CHAR_HEIGHT + 2 * y);
            painter.fill(box_area, Pen::BACKGROUND);
            painter.text(area.x + x, area.y + y, notification, Pen::SHINE);
        }
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
            MainAction::ToggleVideo => state.set_video(state.video().other()),
        }
    }

    fn describe(action: MainAction) -> &'static str {
        return match action {
            MainAction::Quit => "Quit OctaMEDIC",
            MainAction::ToggleVideo => "Switch between PAL and NTSC",
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
    fn right_click_copyright() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        assert!(right_click(&mut view, &mut state));
        assert!(state.displays_off);
        assert!(!state.active_meters().scopes);
        assert!(right_click(&mut view, &mut state));
        assert!(!state.displays_off);
    }

    #[test]
    fn notification_top_left() {
        use rustuition::target::Framebuffer;

        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        let mut draw = |state: &AppState| {
            let mut screen = Framebuffer::new(640, 256);
            view.render(
                state,
                screen.bounds(),
                &mut Painter::new(&mut screen, &state.palette),
            );
            screen
        };
        let before = draw(&state);
        let message = "NO AUDIO: SOMETHING LONG";
        state.notify(message.to_string());
        let after = draw(&state);
        let width = text_width(message) + 4;
        let box_pixels: Vec<[u8; 3]> = (0..10)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| after.pixel(x, y).unwrap())
            .collect();
        assert!(
            box_pixels
                .iter()
                .all(|&pixel| pixel == [0, 0, 0] || pixel == [0xCC; 3])
        );
        let changed = (0..10).any(|y| (0..width).any(|x| before.pixel(x, y) != after.pixel(x, y)));
        assert!(changed);
        assert_eq!(after.pixel(width, 5), before.pixel(width, 5));
    }
}
