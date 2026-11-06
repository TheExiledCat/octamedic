use octamedic_core::{amiga::clock::VideoStandard, data::pattern::MAX_TRACKS};
use rustuition::{
    gadget::Gadget,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{pens, state::AppState},
    framework::{
        event::Event,
        widgets::{
            self, Button, Textbox,
            audio_view::{MAX_LEVEL, MeterLevel},
        },
    },
};

pub const INFO_HEIGHT: i32 = 10;

// memory, block name, song name, NRM
const INFO_WIDTHS: [i32; 4] = [176, 228, 204, 32];
const TEXT_OFFSET: [i32; 2] = [4, 1];

// the E1 graphs, measured off OctaMED Pro 4 from the top left of the panel under the info line
const GRAPH_TOP: i32 = 9;
const GRAPH_HEIGHT: i32 = 32;
const SLOT_WIDTH: i32 = 8;
const AXIS: i32 = GRAPH_HEIGHT;
const TICKS: i32 = AXIS + 1;
const LAST_SLOT_COLUMN: i32 = SLOT_WIDTH - 1;
const OCTAVE: i32 = 12 * SLOT_WIDTH;

// each a rect, how far apart its copies are and how many there are, from the graphs' top left
#[rustfmt::skip]
const SCALES: [(Rect, [i32; 2], usize); 14] = [
    // up the sides and either side of the gap
    (Rect::new(14, 0, 2, GRAPH_HEIGHT), [0, 0], 1),
    (Rect::new(264, 0, 2, GRAPH_HEIGHT), [0, 0], 1),
    (Rect::new(278, 0, 2, GRAPH_HEIGHT), [0, 0], 1),
    (Rect::new(624, 0, 2, GRAPH_HEIGHT), [0, 0], 1),
    // marks every 4 lines on them, right across the gap
    (Rect::new(8, 0, 8, 1), [0, 4], 8),
    (Rect::new(264, 0, 16, 1), [0, 4], 8),
    (Rect::new(624, 0, 8, 1), [0, 4], 8),
    // the axes along the bottom
    (Rect::new(14, AXIS, 252, 1), [0, 0], 1),
    (Rect::new(278, AXIS, 348, 1), [0, 0], 1),
    // a tick at both edges of every slot, longer at every C
    (Rect::new(INSTRUMENTS_X, TICKS, 1, 3), [SLOT_WIDTH, 0], INSTRUMENT_SLOTS),
    (Rect::new(INSTRUMENTS_X + LAST_SLOT_COLUMN, TICKS, 1, 3), [SLOT_WIDTH, 0], INSTRUMENT_SLOTS),
    (Rect::new(NOTES_X, TICKS, 1, 3), [SLOT_WIDTH, 0], NOTE_SLOTS),
    (Rect::new(NOTES_X + LAST_SLOT_COLUMN, TICKS, 1, 3), [SLOT_WIDTH, 0], NOTE_SLOTS),
    (Rect::new(NOTES_X, TICKS, 1, 5), [OCTAVE, 0], 4),
];
const BAR_PENS: [Pen; SLOT_WIDTH as usize] = [
    pens::RED,
    pens::ROSE,
    pens::PINK,
    pens::BLUSH,
    pens::BLUSH,
    pens::PINK,
    pens::ROSE,
    pens::RED,
];

// instruments from 01, notes from C-1
const INSTRUMENT_SLOTS: usize = 31;
const NOTE_SLOTS: usize = 43;
const INSTRUMENTS_X: i32 = 16;
const NOTES_X: i32 = 280;

// bars go up in 4 steps, a full volume note fills them, then they fall like the other meters
const BAR_STEP: i32 = 7;
const MAX_BAR_STEPS: u32 = 4;
const MAX_VOLUME: u32 = 64;

pub struct Footer {
    block_name: Textbox,
    song_name: Textbox,
    // what it switches isn't known yet
    mode: Button,
    instruments: [MeterLevel; INSTRUMENT_SLOTS],
    notes: [MeterLevel; NOTE_SLOTS],
    // each track's trigger count when last looked at
    seen: [u8; MAX_TRACKS],
}

impl Footer {
    pub fn new() -> Self {
        let name_box = || widgets::textbox().padding(1);
        let block_name = name_box().on_update(|state, name| state.set_block_name(name));
        let song_name = name_box().on_update(|state, name| state.set_song_name(name));
        return Self {
            block_name,
            song_name,
            mode: Button::new("NRM").enabled(false),
            instruments: [MeterLevel::default(); INSTRUMENT_SLOTS],
            notes: [MeterLevel::default(); NOTE_SLOTS],
            seen: [0; MAX_TRACKS],
        };
    }

    fn render_info(&mut self, state: &AppState, info: Rect, painter: &mut Painter) {
        let [memory, block_name, song_name, mode] = info.columns(&INFO_WIDTHS)[..] else {
            return;
        };
        let [x, y] = TEXT_OFFSET;
        painter.frame(memory, 1);
        painter.text(
            memory.x + x,
            memory.y + y,
            &memory_text(state),
            Pen::BACKGROUND,
        );
        self.block_name.render(state, block_name, painter);
        self.song_name.render(state, song_name, painter);
        self.mode.render(state, mode, painter);
    }

    fn render_scales(&self, panel: Rect, painter: &mut Painter) {
        let [x, y] = [panel.x, panel.y + GRAPH_TOP];
        for (first, [step_x, step_y], count) in SCALES {
            for copy in 0..count as i32 {
                let rect = Rect::new(
                    x + first.x + copy * step_x,
                    y + first.y + copy * step_y,
                    first.w,
                    first.h,
                );
                painter.fill(rect, Pen::BACKGROUND);
            }
        }
    }

    fn render_bars(&self, panel: Rect, painter: &mut Painter) {
        let axis = panel.y + GRAPH_TOP + GRAPH_HEIGHT;
        let graphs = [
            (INSTRUMENTS_X, &self.instruments[..]),
            (NOTES_X, &self.notes[..]),
        ];
        for (x, slots) in graphs {
            for (slot, meter) in slots.iter().enumerate() {
                let height = bar_steps(meter.level()) as i32 * BAR_STEP;
                let left = panel.x + x + slot as i32 * SLOT_WIDTH;
                for (column, pen) in BAR_PENS.into_iter().enumerate() {
                    let column = Rect::new(left + column as i32, axis - height, 1, height);
                    painter.fill(column, pen);
                }
            }
        }
    }
}

impl Gadget<AppState, Event> for Footer {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // both see every click, it's how a text box finds out it lost focus
        let handled =
            self.block_name.handle_event(state, event) | self.song_name.handle_event(state, event);
        return handled || self.mode.handle_event(state, event);
    }

    fn update(&mut self, state: &mut AppState) {
        if !self.block_name.is_focused() {
            let name = state.active_pattern().map_or("", |pattern| &pattern.name);
            self.block_name.set_text(name);
        }
        if !self.song_name.is_focused() {
            self.song_name.set_text(state.song().name());
        }
        // the graphs are below the bottom of an NTSC screen
        if state.video() == VideoStandard::Ntsc {
            return;
        }
        let mut instruments = [0; INSTRUMENT_SLOTS];
        let mut notes = [0; NOTE_SLOTS];
        let status = state.engine.status();
        for track in 0..MAX_TRACKS {
            let trigger = status.note_trigger(track);
            if trigger.count == self.seen[track] {
                continue;
            }
            self.seen[track] = trigger.count;
            let level = (trigger.volume as u32 * MAX_LEVEL as u32 / MAX_VOLUME) as u8;
            raise(&mut instruments, trigger.instrument, level);
            raise(&mut notes, trigger.note, level);
        }
        for (meter, level) in self.instruments.iter_mut().zip(instruments) {
            meter.update(level);
        }
        for (meter, level) in self.notes.iter_mut().zip(notes) {
            meter.update(level);
        }
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        let (info, panel) = area.split_top(INFO_HEIGHT);
        self.render_info(state, info, painter);
        if state.video() == VideoStandard::Ntsc {
            return;
        }
        painter.frame(panel, 1);
        self.render_scales(panel, painter);
        if state.active_meters().bars {
            self.render_bars(panel, painter);
        }
    }
}

// what has to go in chip memory, then the whole project as a module
fn memory_text(state: &AppState) -> String {
    let project = state.project();
    return format!(
        "Mem  {:7}/{:8}",
        project.chip_bytes(),
        project.module_bytes()
    );
}

fn bar_steps(level: u8) -> u8 {
    return (level as u32 * MAX_BAR_STEPS).div_ceil(MAX_LEVEL as u32) as u8;
}

// slots count from 1, as instruments and notes do
fn raise(slots: &mut [u8], number: u8, level: u8) {
    let slot = (number as usize)
        .checked_sub(1)
        .and_then(|slot| slots.get_mut(slot));
    if let Some(slot) = slot {
        *slot = (*slot).max(level);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{rect::Point, target::Framebuffer};

    use super::*;
    use crate::framework::event::MouseButton;

    const AREA: Rect = Rect::new(0, 197, 640, 59);
    const RED: [u8; 3] = [255, 0, 0];
    const BLACK: [u8; 3] = [0, 0, 0];

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn draw(footer: &mut Footer, state: &AppState) -> Framebuffer {
        let mut screen = Framebuffer::new(640, 256);
        footer.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        return screen;
    }

    #[test]
    fn scales_match_octamed() {
        let screen = draw(&mut Footer::new(), &state());
        for (x, y) in [
            (8, 216),
            (15, 247),
            (264, 244),
            (279, 216),
            (631, 240),
            (14, 248),
        ] {
            assert_eq!(screen.pixel(x, y), Some(BLACK), "{x}, {y}");
        }
        // slot ticks either side of every slot, a longer one at each C
        assert_eq!(screen.pixel(23, 251), Some(BLACK));
        assert_eq!(screen.pixel(24, 251), Some(BLACK));
        assert_ne!(screen.pixel(25, 251), Some(BLACK));
        assert_eq!(screen.pixel(376, 253), Some(BLACK));
        assert_ne!(screen.pixel(384, 253), Some(BLACK));
        assert_ne!(screen.pixel(270, 248), Some(BLACK));
    }

    #[test]
    fn note_raises_instrument_and_note_bars() {
        let mut state = state();
        // F-2 on instrument 02
        state.engine.status().note_triggered(0, 18, 2, 64);
        let mut footer = Footer::new();
        footer.update(&mut state);
        let screen = draw(&mut footer, &state);
        for x in [24, 31, 416, 423] {
            assert_eq!(screen.pixel(x, 247), Some(RED), "x {x}");
            assert_eq!(screen.pixel(x, 220), Some(RED), "x {x}");
            assert_ne!(screen.pixel(x, 219), Some(RED), "x {x}");
        }
        assert_ne!(screen.pixel(16, 247), Some(RED));
    }

    #[test]
    fn bars_fall_like_vu_meters() {
        let mut state = state();
        state.engine.status().note_triggered(0, 1, 1, 32);
        let mut footer = Footer::new();
        footer.update(&mut state);
        assert_eq!(bar_steps(footer.notes[0].level()), 2);
        let mut vu = MeterLevel::default();
        vu.update(64);
        for _ in 0..8 {
            footer.update(&mut state);
            vu.update(0);
            assert_eq!(footer.notes[0], vu);
        }
        assert_eq!(bar_steps(footer.notes[0].level()), 1);
        for _ in 0..8 {
            footer.update(&mut state);
        }
        assert_eq!(footer.notes[0].level(), 0);
    }

    #[test]
    fn e1_hides_bars() {
        let mut state = state();
        state.engine.status().note_triggered(0, 18, 2, 64);
        state.meters.bars = false;
        let mut footer = Footer::new();
        footer.update(&mut state);
        assert_ne!(draw(&mut footer, &state).pixel(24, 247), Some(RED));
    }

    #[test]
    fn typing_renames_song() {
        let mut state = state();
        state.set_song_name("fair");
        let mut footer = Footer::new();
        footer.update(&mut state);
        draw(&mut footer, &state);
        let click = Event::MouseDown {
            pos: Point::new(600, 200),
            button: MouseButton::Left,
        };
        assert!(footer.handle_event(&mut state, &click));
        for character in "light".chars() {
            footer.handle_event(&mut state, &Event::text(character));
        }
        assert_eq!(state.song().name(), "fairlight");
        assert!(!state.is_saved());
        let elsewhere = Event::MouseDown {
            pos: Point::new(300, 100),
            button: MouseButton::Left,
        };
        assert!(!footer.handle_event(&mut state, &elsewhere));
        assert!(!footer.handle_event(&mut state, &Event::text('x')));
    }

    #[test]
    fn typing_renames_block() {
        let mut state = state();
        let mut footer = Footer::new();
        footer.update(&mut state);
        draw(&mut footer, &state);
        let click = Event::MouseDown {
            pos: Point::new(200, 200),
            button: MouseButton::Left,
        };
        assert!(footer.handle_event(&mut state, &click));
        for character in "intro".chars() {
            footer.handle_event(&mut state, &Event::text(character));
        }
        assert_eq!(state.active_pattern().unwrap().name, "intro");
        assert_eq!(state.song().name(), "unnamed");
    }

    #[test]
    fn no_graphs_on_ntsc() {
        let mut state = state();
        state.set_video(VideoStandard::Ntsc);
        state.engine.status().note_triggered(0, 18, 2, 64);
        let mut footer = Footer::new();
        footer.update(&mut state);
        assert_eq!(footer.notes[17].level(), 0);
        let screen = draw(&mut footer, &state);
        assert_ne!(screen.pixel(8, 216), Some(BLACK));
    }
}
