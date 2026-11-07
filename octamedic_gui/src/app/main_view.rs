use ggez::input::keyboard::{KeyCode, KeyMods};
use rustuition::{
    gadget::Gadget,
    painter::{Align, CHAR_HEIGHT, Painter, Pen},
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
        screen_filters::open_filters_dialog,
        state::{AppState, PlayStart},
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

// the notification bar covers the title bar
const NOTIFICATION_TEXT_X: i32 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MainAction {
    Quit,
    ToggleVideo,
    ScreenFilters,
    Play(PlayStart),
    ToggleFilter,
}

impl Action for MainAction {
    // starting again while held would keep restarting
    fn repeats(self) -> bool {
        return !matches!(self, MainAction::Play(_));
    }
}

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
        let keys = Keymap::new("main")
            .bind(Key::ctrl(KeyCode::Q), MainAction::Quit, "Quit")
            // temporary, until there's a setting for it
            .bind(Key::ctrl(KeyCode::F12), MainAction::ToggleVideo, "PAL/NTSC")
            .bind(
                Key::ctrl(KeyCode::F11),
                MainAction::ScreenFilters,
                "Filters",
            )
            .bind(
                Key::amiga(KeyCode::P),
                MainAction::Play(PlayStart::Song),
                "Play song",
            )
            .bind(
                Key::new(KeyCode::Space, KeyMods::SHIFT | KeyMods::ALT),
                MainAction::Play(PlayStart::Song),
                "Play song",
            )
            .bind(
                Key::amiga(KeyCode::Space),
                MainAction::Play(PlayStart::Block),
                "Play block",
            )
            .bind(
                Key::alt(KeyCode::Space),
                MainAction::Play(PlayStart::ContinueBlock),
                "Cont. block",
            );
        let keys = keys.bind(Key::amiga(KeyCode::F), MainAction::ToggleFilter, "Filter");
        return Self {
            keys,
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

    // every panel sees a click, so a text box in one finds out a click in another took its
    // focus. anything else stops at the first that takes it
    fn pass_to_widgets(&mut self, state: &mut AppState, event: &Event) -> bool {
        let mut widgets = self.widgets(state.main_mode()).into_iter();
        if matches!(event, Event::MouseDown { .. }) {
            return widgets.fold(false, |taken, widget| {
                widget.handle_event(state, event) | taken
            });
        }
        return widgets.any(|widget| widget.handle_event(state, event));
    }
}

impl Gadget<AppState, Event> for MainView {
    fn update(&mut self, state: &mut AppState) {
        state.typing = false;
        for widget in self.widgets(state.main_mode()) {
            widget.update(state);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // global keys first, or a child eats them
        let taken = self.console.handle_event(state, event)
            || self.copyright.handle_event(state, event)
            || self.dispatch(state, event)
            || self.pass_to_widgets(state, event);
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
        // last, so a text field being typed in gets its space first. space stops, shift+space
        // continues the song, like the STOP and CONT. SONG buttons
        if let Some(mods) = space_bar_mods(event).filter(|_| !taken) {
            if mods.contains(KeyMods::SHIFT) {
                state.start(PlayStart::ContinueSong);
            } else {
                state.stop();
            }
            return true;
        }
        // any screen plays the keyboard's notes, unless a text field or the block editor took them
        if let Some(key) = typed_key(event).filter(|_| !taken) {
            return state.play_key(state.cursor.track, key).is_some();
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
        let prompt = state.prompt.as_ref().map(|prompt| prompt.text());
        if let Some(text) = prompt.or(state.notification.as_deref()) {
            let bar = Rect::new(area.x, area.y, area.w, TITLE_HEIGHT);
            let y = bar.y + (bar.h - CHAR_HEIGHT + 1) / 2;
            painter.fill(bar, Pen::BACKGROUND);
            painter.text(bar.x + NOTIFICATION_TEXT_X, y, text, Pen::SHINE);
        }
    }
}

// a typed character that isn't part of a shortcut
fn typed_key(event: &Event) -> Option<char> {
    let Event::Text {
        character, mods, ..
    } = *event
    else {
        return None;
    };
    if mods.intersects(KeyMods::CTRL | KeyMods::ALT | KeyMods::LOGO) {
        return None;
    }
    return Some(character);
}

// what's held with a typed space, if it's not a shortcut
fn space_bar_mods(event: &Event) -> Option<KeyMods> {
    let Event::Text {
        character: ' ',
        mods,
        ..
    } = *event
    else {
        return None;
    };
    if mods.intersects(KeyMods::CTRL | KeyMods::ALT | KeyMods::LOGO) {
        return None;
    }
    return Some(mods);
}

impl KeyActions for MainView {
    type Action = MainAction;

    fn keymap(&self) -> &Keymap<MainAction> {
        return &self.keys;
    }

    fn perform(&mut self, action: MainAction, state: &mut AppState) {
        match action {
            MainAction::Quit => state.confirm_if_unsaved(AppState::request_exit),
            MainAction::ToggleVideo => state.set_video(state.video().other()),
            MainAction::ScreenFilters => open_filters_dialog(state),
            MainAction::Play(start) => state.start(start),
            MainAction::ToggleFilter => state.set_filter(!state.song().flags.filter_on),
        }
    }

    fn describe(action: MainAction) -> &'static str {
        return match action {
            MainAction::Quit => "Quit OctaMEDIC",
            MainAction::ToggleVideo => "Switch between PAL and NTSC",
            MainAction::ScreenFilters => "Scanlines and other screen filters",
            MainAction::Play(PlayStart::Song) => "Play the song from the start",
            MainAction::Play(PlayStart::ContinueSong) => "Continue playing the song",
            MainAction::Play(PlayStart::Block) => "Play the block from the start",
            MainAction::Play(PlayStart::ContinueBlock) => "Continue playing the block",
            MainAction::ToggleFilter => "Switch the low pass filter on or off",
        };
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{
        data::{pattern::OctamedicPattern, project::OctamedicProject},
        engine::handle::EngineHandle,
    };
    use rustuition::rect::Point;

    use super::*;

    fn right_click(view: &mut MainView, state: &mut AppState) -> bool {
        let event = Event::MouseDown {
            pos: Point::new(300, 150),
            button: MouseButton::Right,
        };
        return view.handle_event(state, &event);
    }

    #[test]
    fn f1_to_f5_pick_the_octave_on_any_screen() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_modes(
            crate::app::mode::HeaderMode::Instrument,
            MainMode::SampleEditor,
        );
        let mut view = MainView::new();
        let keys = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
        ];
        for (octave, key) in (1..).zip(keys) {
            let event = Event::Key {
                key,
                mods: KeyMods::NONE,
                repeat: false,
            };
            assert!(view.handle_event(&mut state, &event));
            assert_eq!(state.current_octave, octave, "{key:?}");
        }
    }

    #[test]
    fn play_keys() {
        use crate::framework::keymap::AMIGA;

        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        let mut press = |state: &mut AppState, key, mods| {
            let event = Event::Key {
                key,
                mods,
                repeat: false,
            };
            assert!(view.handle_event(state, &event));
            return state.started();
        };
        let space = KeyCode::Space;
        assert_eq!(press(&mut state, KeyCode::P, AMIGA), Some(PlayStart::Song));
        assert_eq!(press(&mut state, space, AMIGA), Some(PlayStart::Block));
        let alt = KeyMods::ALT;
        assert_eq!(
            press(&mut state, space, alt),
            Some(PlayStart::ContinueBlock)
        );
        let shift_alt = KeyMods::SHIFT | KeyMods::ALT;
        assert_eq!(press(&mut state, space, shift_alt), Some(PlayStart::Song));
    }

    #[test]
    fn ctrl_q_asks_about_unsaved_changes() {
        let ctrl_q = Event::Key {
            key: KeyCode::Q,
            mods: KeyMods::CTRL,
            repeat: false,
        };
        let mut view = MainView::new();
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_song_name("changed");
        assert!(view.handle_event(&mut state, &ctrl_q));
        assert!(!state.exit_requested);
        assert!(state.prompt.is_some());

        let mut saved = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        assert!(view.handle_event(&mut saved, &ctrl_q));
        assert!(saved.exit_requested);
    }

    #[test]
    fn amiga_f_switches_the_filter() {
        use crate::framework::keymap::AMIGA;

        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        let before = state.song().flags.filter_on;
        let event = Event::Key {
            key: KeyCode::F,
            mods: AMIGA,
            repeat: false,
        };
        assert!(view.handle_event(&mut state, &event));
        assert_eq!(state.song().flags.filter_on, !before);
    }

    #[test]
    fn keypad_edits_the_sequence() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        for index in 1..=2 {
            state
                .song_mut()
                .insert_pattern(index, OctamedicPattern::new());
        }
        let mut view = MainView::new();
        let mut press = |state: &mut AppState, key| {
            let event = Event::Key {
                key,
                mods: KeyMods::CTRL,
                repeat: false,
            };
            assert!(view.handle_event(state, &event));
        };
        let blocks = |state: &AppState| {
            let sequence = state.song().sequence();
            sequence.iter().map(|block| block.0).collect::<Vec<_>>()
        };
        press(&mut state, KeyCode::Numpad5);
        press(&mut state, KeyCode::Numpad6);
        press(&mut state, KeyCode::Numpad6);
        assert_eq!(blocks(&state), [0, 2]);
        assert_eq!(state.active_sequence_index, 1);
        press(&mut state, KeyCode::Numpad8);
        press(&mut state, KeyCode::Numpad0);
        assert_eq!(blocks(&state), [0, 0, 2]);
        press(&mut state, KeyCode::Numpad2);
        press(&mut state, KeyCode::Numpad4);
        assert_eq!(blocks(&state), [0, 0, 1]);
        press(&mut state, KeyCode::Numpad7);
        assert_eq!(state.active_sequence_index, 0);
        press(&mut state, KeyCode::NumpadDecimal);
        assert_eq!(blocks(&state), [0, 1]);
        press(&mut state, KeyCode::Numpad1);
        assert_eq!(state.active_sequence_index, 1);
    }

    #[test]
    fn caps_lock_keypad_needs_no_ctrl() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        let keypad_5 = Event::Key {
            key: KeyCode::Numpad5,
            mods: KeyMods::NONE,
            repeat: false,
        };
        assert!(!view.handle_event(&mut state, &keypad_5));
        assert_eq!(state.song().sequence().len(), 1);
        state.caps_lock = true;
        assert!(view.handle_event(&mut state, &keypad_5));
        assert_eq!(state.song().sequence().len(), 2);
    }

    #[test]
    fn notes_play_on_any_screen() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_modes(crate::app::mode::HeaderMode::Play, MainMode::SampleEditor);
        let mut view = MainView::new();
        assert!(view.handle_event(&mut state, &Event::text('z')));
        // not a note key
        assert!(!view.handle_event(&mut state, &Event::text('`')));
        let with_ctrl = Event::Text {
            character: 'z',
            mods: KeyMods::CTRL,
            repeat: false,
        };
        assert!(!view.handle_event(&mut state, &with_ctrl));
    }

    #[test]
    fn shift_space_continues_the_song() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut view = MainView::new();
        let shift_space = Event::Text {
            character: ' ',
            mods: KeyMods::SHIFT,
            repeat: false,
        };
        assert!(view.handle_event(&mut state, &shift_space));
        assert_eq!(state.started(), Some(PlayStart::ContinueSong));
        assert!(view.handle_event(&mut state, &Event::text(' ')));
        assert_eq!(state.started(), None);
    }

    #[test]
    fn space_stops_unless_typing() {
        use rustuition::target::Framebuffer;

        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_song_name("ab");
        let mut view = MainView::new();
        assert!(view.handle_event(&mut state, &Event::text(' ')));
        view.update(&mut state);
        let mut screen = Framebuffer::new(640, 256);
        view.render(
            &state,
            screen.bounds(),
            &mut Painter::new(&mut screen, &state.palette),
        );
        let song_name = Event::MouseDown {
            pos: Point::new(600, 200),
            button: MouseButton::Left,
        };
        assert!(view.handle_event(&mut state, &song_name));
        assert!(view.handle_event(&mut state, &Event::text(' ')));
        assert_eq!(state.song().name(), "ab ");
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
    fn notification_bar_across_the_top() {
        use rustuition::{painter::text_width, target::Framebuffer};

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
        // black right across, the text in the background's light grey
        let bar_pixels: Vec<[u8; 3]> = (0..10)
            .flat_map(|y| (0..640).map(move |x| (x, y)))
            .map(|(x, y)| after.pixel(x, y).unwrap())
            .collect();
        assert!(
            bar_pixels
                .iter()
                .all(|&pixel| pixel == [0, 0, 0] || pixel == [0xCC; 3])
        );
        assert_eq!(after.pixel(639, 5), Some([0, 0, 0]));
        let text_end = text_width(message);
        let changed =
            (0..10).any(|y| (0..text_end).any(|x| before.pixel(x, y) != after.pixel(x, y)));
        assert!(changed);
        assert_eq!(after.pixel(10, 10), before.pixel(10, 10));
    }
}
