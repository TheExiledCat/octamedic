use mmd::mmd::module::SONG_TRACKS;
use octamedic_core::data::volume::MAX_VOLUME;
use rustuition::{
    gadget::Gadget,
    gadgets::button::{ButtonContent, ContentArea, Triangle},
    painter::{Painter, Pen},
    rect::Rect,
    shape::Direction::{self, Left, Right},
    text::{Line, Span},
};

use crate::{
    app::{fonts, pens, state::AppState},
    framework::{
        event::{Event, MouseButton},
        widgets::{self, Button, Textbox},
    },
};

// from the panel's top left
const TITLE: [i32; 2] = [4, 3];
const TRACKS: [i32; 2] = [10, 9];
const TRACK_BOX: [i32; 2] = [20, 11];
const TRACK_COLUMNS: usize = 8;
const TRACK_TEXT: [i32; 2] = [3, 2];
const MASTER_LABEL: [[i32; 2]; 2] = [[173, 11], [173, 17]];
const MASTER_ARROW: [i32; 2] = [174, 23];
const INDEX: Rect = Rect::new(10, 32, 20, 10);
const INDEX_TEXT: [i32; 2] = [3, 1];
const COLON: [i32; 2] = [33, 35];
const TRACK_VOLUME: Rect = Rect::new(42, 32, 28, 10);
const MASTER_VOLUME: Rect = Rect::new(159, 32, 28, 10);
const STEP_WIDTH: i32 = 20;
const DIVIDER: Rect = Rect::new(269, 0, 2, 43);
// at the top right of the divider, 90% of the 179px there
const INPUT: Rect = Rect::new(280, 0, 161, 11);
const OUTPUT: Rect = Rect::new(280, 13, 161, 11);

// the arrow down to the master volume box, a span of pixels per line
const ARROW_ROWS: [(i32, i32); 8] = [
    (3, 9),
    (3, 9),
    (0, 12),
    (1, 11),
    (2, 10),
    (3, 9),
    (4, 8),
    (5, 7),
];

const STEPS: [i8; 4] = [-10, -1, 1, 10];

pub struct Volume {
    active_track: usize,
    track_volume: Textbox,
    master_volume: Textbox,
    track_steps: [Button; STEPS.len()],
    master_steps: [Button; STEPS.len()],
    // audio routing settings, not there yet
    input: Button,
    output: Button,
    // from the last render, for clicks
    track_boxes: [Rect; SONG_TRACKS],
}

impl Volume {
    pub fn new() -> Self {
        return Self {
            active_track: 0,
            track_volume: volume_box(),
            master_volume: volume_box(),
            track_steps: step_buttons(),
            master_steps: step_buttons(),
            input: routing_button("INPUT"),
            output: routing_button("OUTPUT"),
            track_boxes: [Rect::default(); SONG_TRACKS],
        };
    }

    fn track_box(area: Rect, track: usize) -> Rect {
        let [x, y] = TRACKS;
        let [w, h] = TRACK_BOX;
        let column = (track % TRACK_COLUMNS) as i32;
        let row = (track / TRACK_COLUMNS) as i32;
        return Rect::new(area.x + x + column * w, area.y + y + row * h, w, h);
    }
}

impl Gadget<AppState, Event> for Volume {
    fn update(&mut self, state: &mut AppState) {
        let track = self.active_track;
        for (step, button) in STEPS.iter().zip(&mut self.track_steps) {
            if button.clicked().is_some() {
                state.set_track_volume(track, stepped(track_volume(state, track), *step));
            }
        }
        for (step, button) in STEPS.iter().zip(&mut self.master_steps) {
            if button.clicked().is_some() {
                state.set_master_volume(stepped(state.song().master_volume.get(), *step));
            }
        }
        let hex = state.song().flags.volumes_hex;
        if let Some(volume) = typed_volume(&self.track_volume, hex) {
            state.set_track_volume(track, volume);
        }
        if let Some(volume) = typed_volume(&self.master_volume, hex) {
            state.set_master_volume(volume);
        }
        if !self.track_volume.is_focused() {
            self.track_volume
                .set_text(volume_text(track_volume(state, track), hex));
        }
        if !self.master_volume.is_focused() {
            self.master_volume
                .set_text(volume_text(state.song().master_volume.get(), hex));
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // both see every click, it's how a text box finds out it lost focus
        if self.track_volume.handle_event(state, event)
            | self.master_volume.handle_event(state, event)
        {
            return true;
        }
        if let Event::MouseDown {
            pos,
            button: MouseButton::Left,
        } = *event
        {
            if let Some(track) = self.track_boxes.iter().position(|area| area.contains(pos)) {
                self.active_track = track;
                return true;
            }
        }
        return self
            .track_steps
            .iter_mut()
            .chain(&mut self.master_steps)
            .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        let label = |text: &'static str| Line::from(Span::new(text).font(fonts::LABEL));
        let [x, y] = TITLE;
        painter.line(
            area.x + x,
            area.y + y,
            &label("THE RELATIVE VOLUMES OF TRACKS 0-15:"),
            Pen::BACKGROUND,
        );
        let hex = state.song().flags.volumes_hex;
        for track in 0..SONG_TRACKS {
            let track_box = Self::track_box(area, track);
            self.track_boxes[track] = track_box;
            painter.frame_inverted(track_box, 1);
            let pen = if track == self.active_track {
                pens::ROSE
            } else {
                Pen::SHINE
            };
            let [x, y] = TRACK_TEXT;
            painter.text_in_font(
                track_box.x + x,
                track_box.y + y,
                &volume_text(track_volume(state, track), hex),
                pen,
                fonts::INTERFACE,
            );
        }
        for ([x, y], text) in MASTER_LABEL.into_iter().zip(["MASTER", "VOLUME"]) {
            painter.line(area.x + x, area.y + y, &label(text), Pen::BACKGROUND);
        }
        let [x, y] = MASTER_ARROW;
        for (row, (left, right)) in ARROW_ROWS.into_iter().enumerate() {
            let line = Rect::new(area.x + x + left, area.y + y + row as i32, right - left, 1);
            painter.fill(line, Pen::BACKGROUND);
        }

        let index = INDEX.within(area);
        painter.frame_inverted(index, 1);
        let [x, y] = INDEX_TEXT;
        painter.text_in_font(
            index.x + x,
            index.y + y,
            &format!("{:02}", self.active_track),
            Pen::BACKGROUND,
            fonts::INTERFACE,
        );
        let [x, y] = COLON;
        for dot in [0, 2] {
            painter.fill(
                Rect::new(area.x + x, area.y + y + dot, 2, 1),
                Pen::BACKGROUND,
            );
        }
        for (textbox, steps, rect) in [
            (&mut self.track_volume, &mut self.track_steps, TRACK_VOLUME),
            (
                &mut self.master_volume,
                &mut self.master_steps,
                MASTER_VOLUME,
            ),
        ] {
            let rect = rect.within(area);
            textbox.render(state, rect, painter);
            painter.frame_inverted(rect, 1);
            for (index, button) in steps.iter_mut().enumerate() {
                let x = rect.right() + index as i32 * STEP_WIDTH;
                button.render(state, Rect::new(x, rect.y, STEP_WIDTH, rect.h), painter);
            }
        }
        painter.fill(DIVIDER.within(area), Pen::BACKGROUND);
        self.input.render(state, INPUT.within(area), painter);
        self.output.render(state, OUTPUT.within(area), painter);
    }
}

fn routing_button(label: &'static str) -> Button {
    return Button::new(label).font(fonts::LABEL).enabled(false);
}

fn volume_box() -> Textbox {
    return widgets::textbox()
        .font(fonts::INTERFACE)
        .padding(1)
        .max_length(2)
        .select_on_focus();
}

fn step_buttons() -> [Button; STEPS.len()] {
    return STEPS.map(|step| {
        let direction = if step < 0 { Left } else { Right };
        if step.abs() > 1 {
            Button::new(double_arrow(direction))
        } else {
            Button::new(Triangle::arrow(direction))
        }
    });
}

// two of the small arrows side by side, for the 10 steps
fn double_arrow(direction: Direction) -> impl Fn(ContentArea, Pen, &mut Painter) {
    let arrow = Triangle::arrow(direction);
    return move |area, pen, painter| {
        let inside = area.inside;
        let left = inside.x + (inside.w - 2 * arrow.width + 1) / 2;
        for copy in 0..2 {
            let x = left + copy * arrow.width;
            let inside = Rect::new(x, inside.y, arrow.width, inside.h);
            arrow.render(ContentArea { inside, ..area }, pen, painter);
        }
    };
}

fn track_volume(state: &AppState, track: usize) -> u8 {
    return state.song().track_volumes[track].get();
}

fn stepped(volume: u8, step: i8) -> u8 {
    return volume.saturating_add_signed(step).min(MAX_VOLUME);
}

fn volume_text(volume: u8, hex: bool) -> String {
    if hex {
        return format!("{volume:02X}");
    }
    return format!("{volume:02}");
}

// only while it's being typed in, and only a volume that's in range
fn typed_volume(textbox: &Textbox, hex: bool) -> Option<u8> {
    if !textbox.is_focused() {
        return None;
    }
    let radix = if hex { 16 } else { 10 };
    return u8::from_str_radix(textbox.text(), radix)
        .ok()
        .filter(|&volume| volume <= MAX_VOLUME);
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{rect::Point, target::Framebuffer};

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 448, 45);

    fn panel() -> (Volume, AppState) {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut volume = Volume::new();
        volume.update(&mut state);
        let mut screen = Framebuffer::new(640, 256);
        volume.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        return (volume, state);
    }

    fn click(volume: &mut Volume, state: &mut AppState, pos: Point) {
        let down = Event::MouseDown {
            pos,
            button: MouseButton::Left,
        };
        let up = Event::MouseUp {
            pos,
            button: MouseButton::Left,
        };
        volume.handle_event(state, &down);
        volume.handle_event(state, &up);
        volume.update(state);
    }

    #[test]
    fn routing_buttons_are_placeholders() {
        let (volume, _) = panel();
        assert!(!volume.input.is_enabled() && !volume.output.is_enabled());
        for rect in [INPUT, OUTPUT] {
            assert!(rect.x > DIVIDER.right() && rect.right() <= AREA.right());
            assert!(rect.bottom() <= AREA.bottom());
        }
    }

    fn centre(rect: Rect) -> Point {
        return Point::new(rect.x + rect.w / 2, rect.y + rect.h / 2);
    }

    #[test]
    fn steps_change_the_chosen_track() {
        let (mut volume, mut state) = panel();
        click(&mut volume, &mut state, centre(Volume::track_box(AREA, 9)));
        assert_eq!(volume.active_track, 9);
        let step_down_ten = volume.track_steps[0].area();
        click(&mut volume, &mut state, centre(step_down_ten));
        let step_up = volume.track_steps[2].area();
        click(&mut volume, &mut state, centre(step_up));
        assert_eq!(track_volume(&state, 9), 55);
        assert_eq!(track_volume(&state, 0), MAX_VOLUME);
        assert_eq!(volume.track_volume.text(), "55");
    }

    #[test]
    fn master_volume_stays_in_range() {
        let (mut volume, mut state) = panel();
        let step_up_ten = volume.master_steps[3].area();
        click(&mut volume, &mut state, centre(step_up_ten));
        assert_eq!(state.song().master_volume.get(), MAX_VOLUME);
        state.set_master_volume(5);
        let step_down_ten = volume.master_steps[0].area();
        click(&mut volume, &mut state, centre(step_down_ten));
        assert_eq!(state.song().master_volume.get(), 0);
    }

    #[test]
    fn typing_a_volume() {
        let (mut volume, mut state) = panel();
        click(&mut volume, &mut state, centre(MASTER_VOLUME.within(AREA)));
        volume.master_volume.set_text("");
        for character in "32".chars() {
            volume.handle_event(&mut state, &Event::text(character));
        }
        volume.update(&mut state);
        assert_eq!(state.song().master_volume.get(), 32);
    }

    #[test]
    fn focusing_a_box_changes_nothing() {
        let (mut volume, mut state) = panel();
        click(&mut volume, &mut state, centre(TRACK_VOLUME.within(AREA)));
        volume.update(&mut state);
        assert!(state.is_saved());
    }

    #[test]
    fn hex_volumes() {
        assert_eq!(volume_text(64, true), "40");
        assert_eq!(volume_text(7, false), "07");
        assert_eq!(stepped(60, 10), MAX_VOLUME);
        assert_eq!(stepped(3, -10), 0);
    }
}
