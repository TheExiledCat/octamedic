use ggez::input::keyboard::{KeyCode, KeyMods};
use octamedic_util::primitives::Flip;
use rustuition::{
    gadget::Gadget,
    painter::{CHAR_HEIGHT, Painter, Pen},
    rect::Rect,
};

use crate::{app::state::AppState, framework::event::Event};

// the key rather than the character, which repeats while held and is a dead key on some layouts.
// with Ctrl, since the key on its own is SP
const TOGGLE: KeyCode = KeyCode::Grave;

const OPACITY: f32 = 0.8;

const MARGIN: [i32; 2] = [4, 2];

#[derive(Default)]
pub struct DebugConsole {
    visible: bool,
}

impl Gadget<AppState, Event> for DebugConsole {
    fn handle_event(&mut self, _state: &mut AppState, event: &Event) -> bool {
        match *event {
            Event::Key {
                key: TOGGLE,
                mods,
                repeat,
            } if mods.contains(KeyMods::CTRL) => {
                if !repeat {
                    self.visible.flip();
                }
                return true;
            }
            _ => return false,
        }
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        if !self.visible {
            return;
        }
        painter.fill_translucent(area, Pen::BACKGROUND, OPACITY);
        let text = area.inset(MARGIN[0], MARGIN[1]);
        let rows = (text.h / CHAR_HEIGHT).max(0) as usize;
        let lines = state.debug_lines();
        let mut y = text.y;
        for line in lines.iter().skip(lines.len().saturating_sub(rows)) {
            painter.text(text.x, y, line, Pen::SHINE);
            y += CHAR_HEIGHT;
        }
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn key(repeat: bool) -> Event {
        return Event::Key {
            key: TOGGLE,
            mods: KeyMods::CTRL,
            repeat,
        };
    }

    #[test]
    fn toggle_ignores_repeat() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut console = DebugConsole::default();
        assert!(console.handle_event(&mut state, &key(false)));
        assert!(console.handle_event(&mut state, &key(true)));
        assert!(console.visible);
        // without Ctrl it's SP's
        let plain = Event::Key {
            key: TOGGLE,
            mods: KeyMods::NONE,
            repeat: false,
        };
        assert!(!console.handle_event(&mut state, &plain));
        console.handle_event(&mut state, &key(false));
        assert!(!console.visible);
    }
}
