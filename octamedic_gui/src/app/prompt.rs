use ggez::input::keyboard::KeyCode;

use crate::{app::state::AppState, framework::event::Event};

// held with another key rather than pressed for themselves
const MODIFIERS: [KeyCode; 9] = [
    KeyCode::LShift,
    KeyCode::RShift,
    KeyCode::LControl,
    KeyCode::RControl,
    KeyCode::LAlt,
    KeyCode::RAlt,
    KeyCode::LWin,
    KeyCode::RWin,
    KeyCode::Capital,
];

type OnKey = Box<dyn FnOnce(&mut AppState, KeyCode)>;

/// A line in the notification bar, the small brother of a dialog. It holds everything up until a
/// key's pressed, then closes and hands that key to its closure to do what it likes with.
pub struct Prompt {
    text: String,
    on_key: OnKey,
}

impl Prompt {
    pub fn new(
        text: impl Into<String>,
        on_key: impl FnOnce(&mut AppState, KeyCode) + 'static,
    ) -> Self {
        return Self {
            text: text.into(),
            on_key: Box::new(on_key),
        };
    }

    /// Does `on_yes` if the answer's Y, and nothing for any other key.
    pub fn yes_no(
        question: impl Into<String>,
        on_yes: impl FnOnce(&mut AppState) + 'static,
    ) -> Self {
        return Self::new(question, |state, key| {
            if key == KeyCode::Y {
                on_yes(state);
            }
        });
    }

    pub fn text(&self) -> &str {
        return &self.text;
    }
}

/// While there's a prompt every event is its. The first key that isn't a modifier answers it.
pub fn handle_event(state: &mut AppState, event: &Event) -> bool {
    if state.prompt.is_none() {
        return false;
    }
    if let Event::Key { key, .. } = *event {
        if !MODIFIERS.contains(&key) {
            if let Some(prompt) = state.prompt.take() {
                (prompt.on_key)(state, key);
            }
        }
    }
    return true;
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::rect::Point;

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn key(key: KeyCode) -> Event {
        return Event::Key {
            key,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
    }

    #[test]
    fn the_first_key_answers() {
        let mut state = state();
        let answer = Rc::new(Cell::new(None));
        let given = answer.clone();
        state.open_prompt(Prompt::new("Which?", move |_, key| given.set(Some(key))));
        let click = Event::MouseDown {
            pos: Point::new(10, 10),
            button: crate::framework::event::MouseButton::Left,
        };
        // everything else is held up, modifiers on their own included
        for event in [click, Event::text('z'), key(KeyCode::LShift)] {
            assert!(handle_event(&mut state, &event));
        }
        assert_eq!(answer.get(), None);
        assert!(handle_event(&mut state, &key(KeyCode::Key3)));
        assert_eq!(answer.get(), Some(KeyCode::Key3));
        assert!(state.prompt.is_none());
        assert!(!handle_event(&mut state, &key(KeyCode::Key4)));
    }

    #[test]
    fn yes_or_anything_else() {
        for (answer, done) in [
            (KeyCode::Y, true),
            (KeyCode::N, false),
            (KeyCode::Escape, false),
        ] {
            let mut state = state();
            let said_yes = Rc::new(Cell::new(false));
            let flag = said_yes.clone();
            state.open_prompt(Prompt::yes_no("Sure? (Y/N)", move |_| flag.set(true)));
            handle_event(&mut state, &key(answer));
            assert_eq!(said_yes.get(), done, "{answer:?}");
        }
    }
}
