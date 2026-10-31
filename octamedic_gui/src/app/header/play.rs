use octamedic_core::data::pattern::PatternId;

use crate::framework::{
    event::MouseButton,
    painter::{Align::Right, CHAR_HEIGHT, Pen},
    rect::Rect,
    shape::Direction::{Down, Up},
    text::Line,
    widget::OctamedicWidget,
    widgets::button::{Button, Triangle},
};
const SEQUENCER_JUMP_BUTTONS: usize = 4;
const SEQUENCER_JUMP_SEPERATOR_ROW: usize = 2;
const JUMP_ARROW_WIDTH: i32 = 12;
const JUMP_ARROW_HEIGHT: i32 = JUMP_ARROW_WIDTH / 2;
const JUMP_BUTTON_WIDTH: i32 = 20;
/// The jump to start and end buttons; the two between them are a line shorter, so all four and
/// the separator are as tall as the list.
const OUTER_JUMP_BUTTON_HEIGHT: i32 = 11;
const INNER_JUMP_BUTTON_HEIGHT: i32 = 10;
const JUMP_SEPARATOR_HEIGHT: i32 = 1;
const SEQUENCE_HEIGHT: i32 = 43;
const SEQUENCE_NUMBERS_RECT: Rect = Rect::new(0, 0, 36, SEQUENCE_HEIGHT);
const SEQUENCE_RECT: Rect = Rect::new(2, 1, 92, 43);
const SEQUENCER_BUTTONS_RECT: Rect = Rect::new(SEQUENCE_NUMBERS_RECT.w, 0, 56, SEQUENCE_HEIGHT);

/// The amount of sequence entries shown on screen at any time
const SEQUENCE_COUNT_MAX: usize = 5;
/// The row of the active entry, with the entries before it above and the ones after below.
const ACTIVE_ROW: usize = SEQUENCE_COUNT_MAX / 2;
/// Blank lines above the first entry, inside the frame.
const SEQUENCE_TOP_MARGIN: i32 = 1;

pub struct Play {
    sequencer: Sequencer,
    playback: Playback,
}
impl Play {
    pub fn new() -> Self {
        return Self {
            sequencer: Sequencer::new(),
            playback: Playback::new(),
        };
    }
}
impl OctamedicWidget for Play {
    fn update(&mut self, state: &mut crate::app::state::AppState) {
        self.sequencer.update(state);
        self.playback.update(state);
    }
    fn handle_event(
        &mut self,
        state: &mut crate::app::state::AppState,
        event: &crate::framework::event::Event,
    ) -> bool {
        return self.sequencer.handle_event(state, event)
            || self.playback.handle_event(state, event);
    }
    fn render(
        &mut self,
        state: &crate::app::state::AppState,
        area: Rect,
        painter: &mut crate::framework::painter::Painter,
    ) {
        self.sequencer
            .render(state, SEQUENCE_RECT.within(area), painter);
    }
}

struct Sequencer {
    jump_buttons: [Button; SEQUENCER_JUMP_BUTTONS],
}
impl Sequencer {
    fn new() -> Self {
        let mut jump_buttons = std::array::from_fn(|_| Button::new(""));
        jump_buttons[0] =
            Button::new(Triangle::new(Up, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT).with_bar())
                .on_click(|state, click| {
                    let MouseButton::Left = click.button else {
                        return;
                    };
                    state.set_sequence(0);
                });
        jump_buttons[1] = Button::new(Triangle::new(Up, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT))
            .on_click(|state, click| {
                let MouseButton::Left = click.button else {
                    return;
                };
                state.prev_sequence();
            });
        jump_buttons[2] = Button::new(Triangle::new(Down, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT))
            .on_click(|state, click| {
                let MouseButton::Left = click.button else {
                    return;
                };
                state.next_sequence();
            });

        jump_buttons[3] =
            Button::new(Triangle::new(Down, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT).with_bar())
                .on_click(|state, click| {
                    let MouseButton::Left = click.button else {
                        return;
                    };
                    state.set_sequence(state.song().sequence().len().saturating_sub(1));
                });
        return Self { jump_buttons };
    }
}
impl OctamedicWidget for Sequencer {
    fn handle_event(
        &mut self,
        state: &mut crate::app::state::AppState,
        event: &crate::framework::event::Event,
    ) -> bool {
        return self
            .jump_buttons
            .iter_mut()
            .any(|button| button.handle_event(state, event));
    }

    fn render(
        &mut self,
        state: &crate::app::state::AppState,
        area: crate::framework::rect::Rect,
        painter: &mut crate::framework::painter::Painter,
    ) {
        let list_rect = SEQUENCE_NUMBERS_RECT.within(area);
        let jump_buttons_rect =
            Rect::new(SEQUENCE_NUMBERS_RECT.right(), 0, JUMP_BUTTON_WIDTH, area.h);
        let sequence = state.song().sequence();
        let active = state.active_sequence_index;
        let previous = active.min(ACTIVE_ROW);
        let next = sequence
            .len()
            .saturating_sub(active + 1)
            .min(SEQUENCE_COUNT_MAX - 1 - ACTIVE_ROW);
        let inside = painter.frame(list_rect, 1);
        let (_, list) = inside.split_top(SEQUENCE_TOP_MARGIN);
        let rows = list.rows(&[CHAR_HEIGHT; SEQUENCE_COUNT_MAX]);
        painter.fill(rows[ACTIVE_ROW], Pen::SHADOW);
        let visible_sequences: Vec<PatternId> = (active - previous..active + next)
            .map(|i| sequence[i])
            .collect();
        for (i, row) in rows[ACTIVE_ROW - previous..=ACTIVE_ROW + next]
            .iter()
            .enumerate()
        {
            painter.line_in(
                *row,
                &Line::from(format!("{:02}", visible_sequences[i.saturating_sub(1)].0).as_str()),
                Pen::LIGHT,
                Right,
            );
        }

        //jump buttons
        let rows = jump_buttons_rect.within(area).rows(&[
            OUTER_JUMP_BUTTON_HEIGHT,
            INNER_JUMP_BUTTON_HEIGHT,
            JUMP_SEPARATOR_HEIGHT,
            INNER_JUMP_BUTTON_HEIGHT,
            OUTER_JUMP_BUTTON_HEIGHT,
        ]);
        painter.fill(rows[SEQUENCER_JUMP_SEPERATOR_ROW], Pen::BLACK);
        for (mut i, button) in self.jump_buttons.iter_mut().enumerate() {
            if i >= SEQUENCER_JUMP_SEPERATOR_ROW {
                i += 1;
            }
            button.render(state, rows[i], painter);
        }
    }
}

struct Playback {}
impl Playback {
    fn new() -> Self {
        return Self {};
    }
}
impl OctamedicWidget for Playback {
    fn render(
        &mut self,
        state: &crate::app::state::AppState,
        area: crate::framework::rect::Rect,
        painter: &mut crate::framework::painter::Painter,
    ) {
        //todo
    }
}
