use octamedic_util::primitives::Flip;
use rustuition::{
    font::Font,
    gadget::Gadget,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{
        fonts,
        header::{
            edit,
            labels::{Words, draw_pixels, draw_words, placed, placed_at},
        },
        state::{AppState, SELECTABLE_TRACKS},
    },
    framework::{
        event::Event,
        widgets::{self, Button, Textbox},
    },
};

// from the panel's top left, measured from OctaMED. The rows are 11px with a 1px gap above the
// fourth
const SELECT_FIRST: Rect = Rect::new(48, 0, 22, 11);
pub const SET: Rect = Rect::new(400, 0, 24, 11);
pub const CLR: Rect = Rect::new(424, 0, 24, 11);

const SELECT_TRACK: Rect = Rect::new(0, 11, 105, 11);
const CUT: Rect = Rect::new(105, 11, 58, 11);
const COPY: Rect = Rect::new(163, 11, 58, 11);
const PASTE: Rect = Rect::new(221, 11, 58, 11);
const PASTE_SELECT: Rect = Rect::new(279, 11, 80, 11);
const CHORD: Rect = Rect::new(359, 11, 64, 11);
const RST: Rect = Rect::new(423, 11, 25, 11);

const SELECT_BLOCK: Rect = Rect::new(0, 22, 105, 11);
const CLEAR: Rect = Rect::new(105, 22, 58, 11);
const CANCEL: Rect = Rect::new(163, 22, 58, 11);
const SPREAD: [Rect; 3] = [
    Rect::new(279, 22, 14, 11),
    Rect::new(293, 22, 14, 11),
    Rect::new(307, 22, 14, 11),
];
const SLIDE_1: Rect = Rect::new(377, 22, 18, 11);
const SLIDE_2: Rect = Rect::new(395, 22, 18, 11);
const SLIDE_VOL: Rect = Rect::new(413, 22, 35, 11);

const NOTE_ECHO: Rect = Rect::new(0, 34, 77, 11);
const DIST: Rect = Rect::new(108, 34, 28, 11);
const MIN: Rect = Rect::new(163, 34, 28, 11);
// the range's first and last track, then its first and last line
const RANGE_BOXES: [Rect; 4] = [
    Rect::new(339, 34, 12, 11),
    Rect::new(361, 34, 12, 11),
    Rect::new(377, 34, 28, 11),
    Rect::new(415, 34, 28, 11),
];
const RANGE_ARROWS: [[i32; 2]; 2] = [[353, 37], [407, 37]];
const RANGE_TEXT: [i32; 2] = [2, 2];

const LABELS: [([i32; 2], Font, Words); 6] = [
    ([4, 3], fonts::NARROW_LABEL, &[(0, "SELECT"), (39, ":")]),
    ([229, 25], fonts::LABEL, &[(0, "SPREAD:")]),
    ([340, 25], fonts::LABEL, &[(0, "SLIDE:")]),
    ([80, 37], fonts::LABEL, &[(0, "DIST")]),
    ([140, 37], fonts::LABEL, &[(0, "M"), (10, "IN")]),
    ([296, 37], fonts::LABEL, &[(0, "RANGE:")]),
];

const DEFAULT_DIST: &str = "4";
const DEFAULT_MIN: &str = "1";

type Action<'a> = (&'a mut Button, Rect, fn(&mut AppState));

pub struct Range {
    select: [Button; SELECTABLE_TRACKS],
    set: Button,
    clr: Button,
    select_track: Button,
    select_block: Button,
    cut: Button,
    copy: Button,
    paste: Button,
    paste_select: Button,
    clear: Button,
    cancel: Button,
    slide_vol: Button,
    chord: Button,
    chord_reset: Button,
    unhooked: Vec<(Button, Rect)>,
    dist: Textbox,
    min: Textbox,
}

impl Range {
    pub fn new() -> Self {
        let label = |words: Words| Button::new(placed(fonts::LABEL, words));
        let unhooked = |font: Font, words: Words, rect: Rect| {
            return (Button::new(placed(font, words)).enabled(false), rect);
        };
        let number_box = |text: &str| {
            let mut field = widgets::textbox().font(fonts::INTERFACE);
            field.set_text(text);
            field.set_enabled(false);
            field
        };
        return Self {
            select: std::array::from_fn(edit::track_digit),
            set: Button::new(placed_at(fonts::TALL, 2, &[(4, "SET")])),
            clr: Button::new(placed_at(fonts::TALL, 2, &[(4, "CLR")])),
            select_track: label(&[(9, "SELECT"), (59, "TRACK")]),
            select_block: label(&[(9, "SELECT"), (59, "BLOCK")]),
            cut: label(&[(19, "CUT")]),
            copy: label(&[(14, "COPY")]),
            paste: label(&[(11, "PASTE")]),
            paste_select: Button::new(placed(fonts::NARROW_LABEL, &[(4, "PASTE"), (40, "SELECT")])),
            clear: label(&[(11, "CLEAR")]),
            cancel: label(&[(7, "CANCEL")]),
            slide_vol: label(&[(7, "VOL")]),
            chord: Button::new(placed(fonts::PANEL, &[(9, "CHORD")])),
            chord_reset: Button::new(placed(fonts::NARROW_LABEL, &[(4, "RST")])),
            unhooked: vec![
                unhooked(fonts::LABEL, &[(5, "2")], SPREAD[0]),
                unhooked(fonts::LABEL, &[(5, "3")], SPREAD[1]),
                unhooked(fonts::LABEL, &[(4, "4")], SPREAD[2]),
                unhooked(fonts::LABEL, &[(7, "1")], SLIDE_1),
                unhooked(fonts::LABEL, &[(6, "2")], SLIDE_2),
                unhooked(fonts::LABEL, &[(5, "NOTE"), (43, "ECHO")], NOTE_ECHO),
            ],
            dist: number_box(DEFAULT_DIST),
            min: number_box(DEFAULT_MIN),
        };
    }

    fn actions(&mut self) -> [Action<'_>; 9] {
        return [
            (
                &mut self.select_track,
                SELECT_TRACK,
                AppState::select_track_range,
            ),
            (
                &mut self.select_block,
                SELECT_BLOCK,
                AppState::select_block_range,
            ),
            (&mut self.cut, CUT, AppState::cut_range),
            (&mut self.copy, COPY, AppState::copy_range),
            (&mut self.paste, PASTE, AppState::paste_range),
            (&mut self.paste_select, PASTE_SELECT, AppState::paste_select),
            (&mut self.clear, CLEAR, AppState::clear_range),
            (&mut self.cancel, CANCEL, AppState::cancel_range),
            (&mut self.slide_vol, SLIDE_VOL, AppState::slide_volume),
        ];
    }

    pub fn select_rect(track: usize) -> Rect {
        let x = SELECT_FIRST.x + track as i32 * SELECT_FIRST.w;
        return Rect::new(x, SELECT_FIRST.y, SELECT_FIRST.w, SELECT_FIRST.h);
    }
}

impl Default for Range {
    fn default() -> Self {
        Self::new()
    }
}

impl Gadget<AppState, Event> for Range {
    fn update(&mut self, state: &mut AppState) {
        for (track, button) in self.select.iter_mut().enumerate() {
            if button.clicked().is_some() {
                state.selected_tracks[track] = !state.selected_tracks[track];
            }
        }
        if self.set.clicked().is_some() {
            state.selected_tracks = [true; SELECTABLE_TRACKS];
        }
        if self.clr.clicked().is_some() {
            state.selected_tracks = [false; SELECTABLE_TRACKS];
        }
        for (button, selected) in self.select.iter_mut().zip(state.selected_tracks) {
            button.set_toggled(selected);
        }
        if self.chord.clicked().is_some() {
            state.chord_mode.flip();
        }
        if self.chord_reset.clicked().is_some() {
            state.chord_reset.flip();
        }
        self.chord.set_toggled(state.chord_mode);
        self.chord_reset.set_toggled(state.chord_reset);
        for (button, _, action) in self.actions() {
            if button.clicked().is_some() {
                action(state);
            }
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let selecting = self
            .select
            .iter_mut()
            .chain([
                &mut self.set,
                &mut self.clr,
                &mut self.chord,
                &mut self.chord_reset,
            ])
            .any(|button| button.handle_event(state, event));
        return selecting
            || self
                .actions()
                .into_iter()
                .any(|(button, _, _)| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for ([x, y], font, words) in LABELS {
            draw_words(
                painter,
                font,
                [area.x + x, area.y + y],
                words,
                Pen::BACKGROUND,
            );
        }
        for (track, button) in self.select.iter_mut().enumerate() {
            button.render(state, Self::select_rect(track).within(area), painter);
        }
        self.set.render(state, SET.within(area), painter);
        self.clr.render(state, CLR.within(area), painter);
        for (button, rect, _) in self.actions() {
            button.render(state, rect.within(area), painter);
        }
        self.chord.render(state, CHORD.within(area), painter);
        self.chord_reset.render(state, RST.within(area), painter);
        for (button, rect) in &mut self.unhooked {
            button.render(state, rect.within(area), painter);
        }
        self.dist.render(state, DIST.within(area), painter);
        self.min.render(state, MIN.within(area), painter);
        render_range(state, area, painter);
    }
}

fn render_range(state: &AppState, area: Rect, painter: &mut Painter) {
    let numbers = state.active_range().map(|range| {
        let (tracks, lines) = (range.tracks(), range.lines());
        return [
            format!("{:X}", tracks.start),
            format!("{:X}", tracks.end - 1),
            format!("{:03}", lines.start),
            format!("{:03}", lines.end - 1),
        ];
    });
    for (index, rect) in RANGE_BOXES.iter().enumerate() {
        let rect = rect.within(area);
        let inside = painter.frame_inverted(rect, 1);
        if let Some(numbers) = &numbers {
            let [x, y] = RANGE_TEXT;
            let text = &numbers[index];
            // tracks past F take two digits, more than the track boxes have room for
            painter.clipped(inside, |painter| {
                painter.text_in_font(
                    rect.x + x,
                    rect.y + y,
                    text,
                    Pen::BACKGROUND,
                    fonts::INTERFACE,
                );
            });
        }
    }
    for [x, y] in RANGE_ARROWS {
        draw_pixels(
            painter,
            [area.x + x, area.y + y],
            &RANGE_ARROW,
            Pen::BACKGROUND,
        );
    }
}

const RANGE_ARROW: [&str; 5] = ["##....", "####..", "######", "####..", "##...."];

// OctaMED's own digits, taller than any of the fonts
#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{event::MouseButton, rect::Point, target::Framebuffer};

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 448, 45);

    fn all_rects() -> Vec<Rect> {
        return (0..SELECTABLE_TRACKS)
            .map(Range::select_rect)
            .chain([
                SET,
                CLR,
                SELECT_TRACK,
                CUT,
                COPY,
                PASTE,
                PASTE_SELECT,
                CHORD,
                RST,
                SELECT_BLOCK,
                CLEAR,
                CANCEL,
                SLIDE_1,
                SLIDE_2,
                SLIDE_VOL,
                NOTE_ECHO,
                DIST,
                MIN,
            ])
            .chain(SPREAD)
            .chain(RANGE_BOXES)
            .collect();
    }

    #[test]
    fn everything_fits_without_overlapping() {
        let rects = all_rects();
        for (i, a) in rects.iter().enumerate() {
            let inside = a.x >= AREA.x && a.y >= AREA.y && a.right() <= AREA.right();
            assert!(
                inside && a.bottom() <= AREA.bottom(),
                "{a:?} is outside the panel"
            );
            for b in &rects[i + 1..] {
                let apart = a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y;
                assert!(apart || b.bottom() <= a.y, "{a:?} overlaps {b:?}");
            }
        }
    }

    fn click(panel: &mut Range, state: &mut AppState, rect: Rect) {
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        panel.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let pos = Point::new(rect.x + 2, rect.y + 2);
        let button = MouseButton::Left;
        panel.handle_event(state, &Event::MouseDown { pos, button });
        panel.handle_event(state, &Event::MouseUp { pos, button });
        panel.update(state);
    }

    #[test]
    fn select_buttons_pick_tracks() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut panel = Range::new();
        click(&mut panel, &mut state, CLR);
        assert_eq!(state.selected_tracks, [false; SELECTABLE_TRACKS]);
        click(&mut panel, &mut state, Range::select_rect(3));
        assert!(state.selected_tracks[3] && panel.select[3].is_toggled());
        click(&mut panel, &mut state, Range::select_rect(3));
        assert!(!state.selected_tracks[3]);
        click(&mut panel, &mut state, SET);
        assert_eq!(state.selected_tracks, [true; SELECTABLE_TRACKS]);
    }

    #[test]
    fn chord_and_rst_switch_on_and_off() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut panel = Range::new();
        click(&mut panel, &mut state, CHORD);
        click(&mut panel, &mut state, RST);
        assert!(state.chord_mode && panel.chord.is_toggled());
        assert!(state.chord_reset && panel.chord_reset.is_toggled());
        click(&mut panel, &mut state, CHORD);
        assert!(!state.chord_mode);
    }

    #[test]
    fn buttons_work_on_the_range() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut panel = Range::new();
        state.active_pattern_mut().unwrap().lines[0].tracks[0]
            .note
            .value = 13;
        click(&mut panel, &mut state, SELECT_TRACK);
        click(&mut panel, &mut state, CUT);
        assert_eq!(
            state.active_pattern().unwrap().lines[0].tracks[0]
                .note
                .value,
            0
        );
        state.cursor.track = 1;
        click(&mut panel, &mut state, PASTE);
        assert_eq!(
            state.active_pattern().unwrap().lines[0].tracks[1]
                .note
                .value,
            13
        );
        click(&mut panel, &mut state, CANCEL);
        assert_eq!(state.active_range(), None);
        click(&mut panel, &mut state, SELECT_BLOCK);
        assert_eq!(state.active_range().unwrap().tracks(), 0..4);
    }

    #[test]
    fn the_rest_are_ghosted() {
        let panel = Range::new();
        assert!(
            panel
                .unhooked
                .iter()
                .all(|(button, _)| !button.is_enabled())
        );
    }
}
