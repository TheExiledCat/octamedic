use rustuition::{gadget::Gadget, painter::Painter, rect::Rect};

use crate::{
    app::state::AppState,
    framework::{event::Event, widgets::Dialog},
};

/// The open dialogs, newest on top. The top one gets every event.
#[derive(Default)]
pub struct DialogStack {
    dialogs: Vec<Dialog>,
}

impl DialogStack {
    pub fn is_empty(&self) -> bool {
        return self.dialogs.is_empty();
    }

    // closed ones go, ones opened through the state come in
    pub fn update(&mut self, state: &mut AppState) {
        for dialog in &mut self.dialogs {
            dialog.update(state);
        }
        self.dialogs.retain(|dialog| !dialog.is_closed());
        self.dialogs.extend(state.take_pending_dialogs());
    }

    /// False when there's no dialog open, so the screen underneath gets the event.
    pub fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let Some(top) = self.dialogs.last_mut() else {
            return false;
        };
        top.handle_event(state, event);
        return true;
    }

    pub fn render(&mut self, state: &AppState, screen: Rect, painter: &mut Painter) {
        for dialog in &mut self.dialogs {
            dialog.render(state, screen, painter);
        }
    }
}

#[cfg(test)]
mod tests {
    use ggez::input::keyboard::{KeyCode, KeyMods};
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn key(key: KeyCode) -> Event {
        return Event::Key {
            key,
            mods: KeyMods::NONE,
            repeat: false,
        };
    }

    #[test]
    fn opened_dialogs_show_up_and_closed_ones_go() {
        let mut state = state();
        let mut stack = DialogStack::default();
        state.open_dialog(Dialog::confirm("Sure?", "Really?"));
        assert!(stack.is_empty());
        stack.update(&mut state);
        assert!(!stack.is_empty());
        assert!(stack.handle_event(&mut state, &key(KeyCode::Escape)));
        stack.update(&mut state);
        assert!(stack.is_empty());
    }

    #[test]
    fn events_pass_through_with_no_dialog() {
        let mut state = state();
        let mut stack = DialogStack::default();
        assert!(!stack.handle_event(&mut state, &key(KeyCode::Escape)));
    }

    #[test]
    fn only_the_top_dialog_gets_events() {
        let mut state = state();
        let mut stack = DialogStack::default();
        let named = |name: &'static str| {
            Dialog::confirm(name, "").on_submit(move |state: &mut AppState, _| {
                state.set_song_name(name);
            })
        };
        state.open_dialog(named("bottom"));
        state.open_dialog(named("top"));
        stack.update(&mut state);
        stack.handle_event(&mut state, &key(KeyCode::Return));
        assert_eq!(state.song().name(), "top");
        stack.update(&mut state);
        stack.handle_event(&mut state, &key(KeyCode::Return));
        assert_eq!(state.song().name(), "bottom");
    }

    #[test]
    fn a_dialog_can_open_another() {
        let mut state = state();
        let mut stack = DialogStack::default();
        let first = Dialog::confirm("First", "").on_submit(|state: &mut AppState, _| {
            state.open_dialog(Dialog::message("Second", ""));
        });
        state.open_dialog(first);
        stack.update(&mut state);
        stack.handle_event(&mut state, &key(KeyCode::Return));
        stack.update(&mut state);
        assert!(!stack.is_empty());
    }
}
