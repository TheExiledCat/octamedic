use octamedic_core::data::pattern::MAX_LINES;
use rustuition::{
    gadget::Gadget,
    gadgets::button::{DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL, Triangle},
    painter::{Painter, Pen},
    rect::Rect,
    shape::Direction,
};

use crate::{
    app::{fonts, pens, state::AppState},
    framework::{
        event::Event,
        widgets::{Button, Textbox, ToggleGroup},
    },
};

// from the panel's top left, one 11px row per TRK:/BLK:/EXPAND/NEW BLOCK line
const TRK_LABEL: [i32; 2] = [2, 2];
const CUT: Rect = Rect::new(28, 0, 28, 11);
const COPY: Rect = Rect::new(57, 0, 36, 11);
const PASTE: Rect = Rect::new(94, 0, 44, 11);
const SWAP: Rect = Rect::new(139, 0, 38, 11);
const INS_TRK: Rect = Rect::new(178, 0, 52, 11);
const TRKS_LABEL: [i32; 2] = [236, 2];
const TRK_COUNT_4: Rect = Rect::new(268, 0, 18, 11);
const TRK_COUNT_8: Rect = Rect::new(287, 0, 18, 11);
const TRK_COUNT_12: Rect = Rect::new(306, 0, 24, 11);
const TRK_COUNT_16: Rect = Rect::new(331, 0, 24, 11);

const BLK_LABEL: [i32; 2] = [2, 13];
const CUT_BLOCK: Rect = Rect::new(28, 11, 28, 11);
const COPY_BLOCK: Rect = Rect::new(57, 11, 36, 11);
const PASTE_BLOCK: Rect = Rect::new(94, 11, 44, 11);
const SWAP_BLOCK: Rect = Rect::new(139, 11, 38, 11);
const DEL_TRK: Rect = Rect::new(178, 11, 52, 11);
const LINES_LABEL: [i32; 2] = [236, 13];
const LINES_BOX: Rect = Rect::new(270, 11, 36, 11);
const LINES_DEC: Rect = Rect::new(307, 11, 15, 11);
const LINES_INC: Rect = Rect::new(323, 11, 15, 11);
const LINES_DEC_10: Rect = Rect::new(339, 11, 19, 11);
const LINES_INC_10: Rect = Rect::new(359, 11, 19, 11);

const EXPAND: Rect = Rect::new(2, 22, 50, 11);
const EXPAND_FACTOR: Rect = Rect::new(53, 22, 24, 11);
const SHRINK: Rect = Rect::new(78, 22, 46, 11);
const INS_LINE: Rect = Rect::new(130, 22, 40, 11);
const DEL_LINE: Rect = Rect::new(171, 22, 40, 11);
const DEL_BUFF: Rect = Rect::new(212, 22, 56, 11);
const HIGHLIGHT: [Rect; 7] = [
    Rect::new(269, 22, 16, 11),
    Rect::new(286, 22, 16, 11),
    Rect::new(303, 22, 16, 11),
    Rect::new(320, 22, 16, 11),
    Rect::new(337, 22, 16, 11),
    Rect::new(354, 22, 22, 11),
    Rect::new(377, 22, 16, 11),
];
// the spacing each HIGHLIGHT button sets, C (last) clears it instead
const HIGHLIGHT_SPACING: [usize; 6] = [1, 2, 3, 4, 8, 16];

const NEW_BLOCK: Rect = Rect::new(2, 33, 72, 11);
const NEW_BLOCK_DEF: Rect = Rect::new(75, 33, 28, 11);
const NEW_HERE: Rect = Rect::new(104, 33, 64, 11);
const NEW_HERE_DEF: Rect = Rect::new(169, 33, 28, 11);
const DELETE_LAST: Rect = Rect::new(198, 33, 84, 11);
const DELETE_THIS: Rect = Rect::new(283, 33, 84, 11);
const SPLIT: Rect = Rect::new(368, 33, 34, 11);
const JOIN: Rect = Rect::new(403, 33, 28, 11);

const TRACK_COUNTS: [usize; 4] = [4, 8, 12, 16];

const LINES_ARROW: [i32; 2] = [6, 5];
const LINES_ARROW_BAR_GAP: i32 = 2;

pub struct Block {
    // track/block clipboard operations aren't implemented yet, so these stay disabled
    cut_track: Button,
    copy_track: Button,
    paste_track: Button,
    swap_track: Button,
    cut_block: Button,
    copy_block: Button,
    paste_block: Button,
    swap_block: Button,
    del_buff: Button,

    ins_track: Button,
    del_track: Button,
    track_counts: [Button; 4],

    lines_box: Textbox,
    lines_box_was_focused: bool,
    lines_dec: Button,
    lines_inc: Button,
    lines_dec_10: Button,
    lines_inc_10: Button,

    expand: Button,
    expand_factor: Textbox,
    shrink: Button,
    ins_line: Button,
    del_line: Button,
    highlight: [Button; 7],

    new_block: Button,
    new_block_def: Button,
    new_here: Button,
    new_here_def: Button,
    delete_last: Button,
    delete_this: Button,
    split: Button,
    join: Button,
}

impl Block {
    pub fn new() -> Self {
        let unhooked = |button: Button| button.enabled(false);
        let narrow = |label: &'static str| Button::new(label).font(fonts::NARROW_LABEL);
        let number_box = || {
            Textbox::new()
                .pens(Pen::BACKGROUND, Pen::SHINE, Pen::FILL, pens::RED)
                .padding(0)
        };
        return Self {
            cut_track: unhooked(narrow("CUT")),
            copy_track: unhooked(narrow("COPY")),
            paste_track: unhooked(narrow("PASTE")),
            swap_track: unhooked(narrow("SWAP")),
            cut_block: unhooked(narrow("CUT")),
            copy_block: unhooked(narrow("COPY")),
            paste_block: unhooked(narrow("PASTE")),
            swap_block: unhooked(narrow("SWAP")),
            del_buff: unhooked(narrow("DEL BUFF")),

            ins_track: narrow("INS.TRK"),
            del_track: narrow("DEL.TRK"),
            // 5-8 channel mode (8/12/16 tracks) isn't supported yet, so there's nothing to pick
            track_counts: TRACK_COUNTS.map(|count| unhooked(narrow(track_count_label(count)))),

            lines_box: number_box(),
            lines_box_was_focused: false,
            lines_dec: step_button(Direction::Left, false),
            lines_inc: step_button(Direction::Right, false),
            lines_dec_10: step_button(Direction::Left, true),
            lines_inc_10: step_button(Direction::Right, true),

            expand: narrow("EXPAND"),
            expand_factor: {
                let mut field = number_box();
                field.set_text("2");
                field
            },
            shrink: narrow("SHRINK"),
            ins_line: narrow("INS.L."),
            del_line: narrow("DEL.L."),
            highlight: HIGHLIGHT_LABELS.map(narrow),

            new_block: narrow("NEW BLOCK"),
            new_block_def: narrow("DEF"),
            new_here: narrow("NEW HERE"),
            new_here_def: narrow("DEF"),
            delete_last: narrow("DELETE LAST"),
            delete_this: narrow("DELETE THIS"),
            split: narrow("SPLIT"),
            join: narrow("JOIN"),
        };
    }

    fn buttons(&mut self) -> impl Iterator<Item = &mut Button> {
        return [
            &mut self.cut_track,
            &mut self.copy_track,
            &mut self.paste_track,
            &mut self.swap_track,
            &mut self.cut_block,
            &mut self.copy_block,
            &mut self.paste_block,
            &mut self.swap_block,
            &mut self.del_buff,
            &mut self.ins_track,
            &mut self.del_track,
            &mut self.lines_dec,
            &mut self.lines_inc,
            &mut self.lines_dec_10,
            &mut self.lines_inc_10,
            &mut self.expand,
            &mut self.shrink,
            &mut self.ins_line,
            &mut self.del_line,
            &mut self.new_block,
            &mut self.new_block_def,
            &mut self.new_here,
            &mut self.new_here_def,
            &mut self.delete_last,
            &mut self.delete_this,
            &mut self.split,
            &mut self.join,
        ]
        .into_iter()
        .chain(&mut self.track_counts)
        .chain(&mut self.highlight);
    }
}

const HIGHLIGHT_LABELS: [&str; 7] = ["1", "2", "3", "4", "8", "16", "C"];

// bar: the fast (+/-10) variant, like the real OctaMED's double arrows
fn step_button(direction: Direction, bar: bool) -> Button {
    let [w, h] = LINES_ARROW;
    let mut triangle = Triangle::new(direction, w, h);
    if bar {
        triangle = triangle.with_bar().bar_gap(LINES_ARROW_BAR_GAP);
    }
    return Button::new(triangle).repeat(DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL);
}

fn track_count_label(count: usize) -> &'static str {
    return match count {
        4 => "4",
        8 => "8",
        12 => "12",
        _ => "16",
    };
}

impl Gadget<AppState, Event> for Block {
    fn update(&mut self, state: &mut AppState) {
        if self.ins_track.clicked().is_some() {
            let track = state.cursor.track;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.insert_track(track);
            }
        }
        if self.del_track.clicked().is_some() {
            let track = state.cursor.track;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.delete_track(track);
            }
        }
        let active_tracks = state.active_pattern().map_or(0, |p| p.track_count());
        let mut track_count_group =
            ToggleGroup::new(self.track_counts.iter_mut().zip(TRACK_COUNTS));
        if let Some((count, _)) = track_count_group.take_click() {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_track_count(count);
            }
            clamp_cursor(state);
        }
        track_count_group.sync(Some(active_tracks));

        if !self.lines_box.is_focused() {
            let lines = state.active_pattern().map_or(0, |p| p.line_count());
            self.lines_box.set_text(lines.to_string());
        }
        if let Some(lines) = finished_number(&self.lines_box, &mut self.lines_box_was_focused) {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_line_count(lines.max(1));
            }
            clamp_cursor(state);
        }
        let step_lines = |state: &mut AppState, delta: isize| {
            let lines = state.active_pattern().map_or(1, |p| p.line_count()) as isize;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_line_count((lines + delta).max(1) as usize);
            }
            clamp_cursor(state);
        };
        if self.lines_dec.clicked().is_some() {
            step_lines(state, -1);
        }
        if self.lines_inc.clicked().is_some() {
            step_lines(state, 1);
        }
        if self.lines_dec_10.clicked().is_some() {
            step_lines(state, -10);
        }
        if self.lines_inc_10.clicked().is_some() {
            step_lines(state, 10);
        }

        let expand_factor = self.expand_factor.text().trim().parse().unwrap_or(2).max(1);
        if self.expand.clicked().is_some() {
            let lines = state.active_pattern().map_or(0, |p| p.line_count());
            if lines * expand_factor > MAX_LINES {
                state.notify("Too many lines");
            } else if let Some(pattern) = state.active_pattern_mut() {
                pattern.expand(expand_factor);
            }
        }
        if self.shrink.clicked().is_some() {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.shrink(expand_factor);
            }
            clamp_cursor(state);
        }

        if self.ins_line.clicked().is_some() {
            let line = state.cursor.line;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.insert_line(line);
            }
        }
        if self.del_line.clicked().is_some() {
            let line = state.cursor.line;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.delete_line(line);
            }
            clamp_cursor(state);
        }

        for (button, spacing) in self.highlight.iter_mut().zip(HIGHLIGHT_SPACING) {
            if button.clicked().is_some() {
                set_highlight_spacing(state, Some(spacing));
            }
        }
        if self.highlight[HIGHLIGHT_SPACING.len()].clicked().is_some() {
            set_highlight_spacing(state, None);
        }

        if self.new_block.clicked().is_some() {
            state.new_block();
        }
        if self.new_block_def.clicked().is_some() {
            state.new_default_block();
        }
        if self.new_here.clicked().is_some() {
            state.new_block_here();
        }
        if self.new_here_def.clicked().is_some() {
            state.new_default_block_here();
        }
        if self.delete_last.clicked().is_some() {
            state.delete_last_block();
        }
        if self.delete_this.clicked().is_some() {
            state.delete_this_block();
        }
        if self.split.clicked().is_some() {
            state.split_block();
        }
        if self.join.clicked().is_some() {
            state.join_block();
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.lines_box.handle_event(state, event)
            || self.expand_factor.handle_event(state, event)
            || self
                .buttons()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        draw_label(painter, area, TRK_LABEL, "TRK:");
        self.cut_track.render(state, CUT.within(area), painter);
        self.copy_track.render(state, COPY.within(area), painter);
        self.paste_track.render(state, PASTE.within(area), painter);
        self.swap_track.render(state, SWAP.within(area), painter);
        self.ins_track.render(state, INS_TRK.within(area), painter);
        draw_label(painter, area, TRKS_LABEL, "TRKS:");
        let track_count_rects = [TRK_COUNT_4, TRK_COUNT_8, TRK_COUNT_12, TRK_COUNT_16];
        for (button, rect) in self.track_counts.iter_mut().zip(track_count_rects) {
            button.render(state, rect.within(area), painter);
        }

        draw_label(painter, area, BLK_LABEL, "BLK:");
        self.cut_block
            .render(state, CUT_BLOCK.within(area), painter);
        self.copy_block
            .render(state, COPY_BLOCK.within(area), painter);
        self.paste_block
            .render(state, PASTE_BLOCK.within(area), painter);
        self.swap_block
            .render(state, SWAP_BLOCK.within(area), painter);
        self.del_track.render(state, DEL_TRK.within(area), painter);
        draw_label(painter, area, LINES_LABEL, "LINES:");
        self.lines_box
            .render(state, LINES_BOX.within(area), painter);
        self.lines_dec
            .render(state, LINES_DEC.within(area), painter);
        self.lines_inc
            .render(state, LINES_INC.within(area), painter);
        self.lines_dec_10
            .render(state, LINES_DEC_10.within(area), painter);
        self.lines_inc_10
            .render(state, LINES_INC_10.within(area), painter);

        self.expand.render(state, EXPAND.within(area), painter);
        self.expand_factor
            .render(state, EXPAND_FACTOR.within(area), painter);
        self.shrink.render(state, SHRINK.within(area), painter);
        self.ins_line.render(state, INS_LINE.within(area), painter);
        self.del_line.render(state, DEL_LINE.within(area), painter);
        self.del_buff.render(state, DEL_BUFF.within(area), painter);
        for (button, rect) in self.highlight.iter_mut().zip(HIGHLIGHT) {
            button.render(state, rect.within(area), painter);
        }

        self.new_block
            .render(state, NEW_BLOCK.within(area), painter);
        self.new_block_def
            .render(state, NEW_BLOCK_DEF.within(area), painter);
        self.new_here.render(state, NEW_HERE.within(area), painter);
        self.new_here_def
            .render(state, NEW_HERE_DEF.within(area), painter);
        self.delete_last
            .render(state, DELETE_LAST.within(area), painter);
        self.delete_this
            .render(state, DELETE_THIS.within(area), painter);
        self.split.render(state, SPLIT.within(area), painter);
        self.join.render(state, JOIN.within(area), painter);
    }
}

fn draw_label(painter: &mut Painter, area: Rect, [x, y]: [i32; 2], text: &str) {
    painter.text_in_font(
        area.x + x,
        area.y + y,
        text,
        Pen::BACKGROUND,
        fonts::NARROW_LABEL,
    );
}

fn clamp_cursor(state: &mut AppState) {
    let (lines, tracks) = state.active_pattern().map_or((1, 1), |pattern| {
        (pattern.line_count(), pattern.track_count())
    });
    state.cursor.clamp(lines, tracks);
}

fn set_highlight_spacing(state: &mut AppState, spacing: Option<usize>) {
    if let Some(pattern) = state.active_pattern_mut() {
        for (index, line) in pattern.lines.iter_mut().enumerate() {
            line.highlighted = spacing.is_some_and(|spacing| index % spacing == 0);
        }
    }
}

// committed on Enter or on losing focus, like the Files panel's DIR/FILE boxes
fn finished_number(textbox: &Textbox, was_focused: &mut bool) -> Option<usize> {
    let left = *was_focused && !textbox.is_focused();
    *was_focused = textbox.is_focused();
    return left.then(|| textbox.text().trim().parse().ok()).flatten();
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{event::MouseButton, rect::Point, target::Framebuffer};

    use super::*;

    // the header's PANEL is 452x47, with a 1px inverted frame eating into that on every side
    const PANEL_INSIDE: Rect = Rect::new(0, 0, 450, 45);
    const AREA: Rect = Rect::new(0, 0, 460, 50);

    fn all_rects() -> Vec<(&'static str, Rect)> {
        return vec![
            ("CUT", CUT),
            ("COPY", COPY),
            ("PASTE", PASTE),
            ("SWAP", SWAP),
            ("INS_TRK", INS_TRK),
            ("TRK_COUNT_4", TRK_COUNT_4),
            ("TRK_COUNT_8", TRK_COUNT_8),
            ("TRK_COUNT_12", TRK_COUNT_12),
            ("TRK_COUNT_16", TRK_COUNT_16),
            ("CUT_BLOCK", CUT_BLOCK),
            ("COPY_BLOCK", COPY_BLOCK),
            ("PASTE_BLOCK", PASTE_BLOCK),
            ("SWAP_BLOCK", SWAP_BLOCK),
            ("DEL_TRK", DEL_TRK),
            ("LINES_BOX", LINES_BOX),
            ("LINES_DEC", LINES_DEC),
            ("LINES_INC", LINES_INC),
            ("LINES_DEC_10", LINES_DEC_10),
            ("LINES_INC_10", LINES_INC_10),
            ("EXPAND", EXPAND),
            ("EXPAND_FACTOR", EXPAND_FACTOR),
            ("SHRINK", SHRINK),
            ("INS_LINE", INS_LINE),
            ("DEL_LINE", DEL_LINE),
            ("DEL_BUFF", DEL_BUFF),
            ("NEW_BLOCK", NEW_BLOCK),
            ("NEW_BLOCK_DEF", NEW_BLOCK_DEF),
            ("NEW_HERE", NEW_HERE),
            ("NEW_HERE_DEF", NEW_HERE_DEF),
            ("DELETE_LAST", DELETE_LAST),
            ("DELETE_THIS", DELETE_THIS),
            ("SPLIT", SPLIT),
            ("JOIN", JOIN),
        ]
        .into_iter()
        .chain(
            HIGHLIGHT
                .into_iter()
                .zip(HIGHLIGHT_LABELS)
                .map(|(rect, label)| (label, rect)),
        )
        .collect();
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        return a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom();
    }

    #[test]
    fn every_button_fits_inside_the_panel() {
        for (name, rect) in all_rects() {
            assert!(rect.x >= PANEL_INSIDE.x, "{name} starts left of the panel");
            assert!(rect.y >= PANEL_INSIDE.y, "{name} starts above the panel");
            assert!(
                rect.right() <= PANEL_INSIDE.right(),
                "{name} ({rect:?}) runs past the panel's right edge ({})",
                PANEL_INSIDE.right()
            );
            assert!(
                rect.bottom() <= PANEL_INSIDE.bottom(),
                "{name} ({rect:?}) runs past the panel's bottom edge ({})",
                PANEL_INSIDE.bottom()
            );
        }
    }

    #[test]
    fn no_two_buttons_overlap() {
        let rects = all_rects();
        for (i, &(name_a, a)) in rects.iter().enumerate() {
            for &(name_b, b) in &rects[i + 1..] {
                assert!(
                    !overlaps(a, b),
                    "{name_a} ({a:?}) overlaps {name_b} ({b:?})"
                );
            }
        }
    }

    fn new_state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn click(block: &mut Block, state: &mut AppState, rect: Rect) {
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        block.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let pos = Point::new(rect.x + 2, rect.y + 2);
        let down = Event::MouseDown {
            pos,
            button: MouseButton::Left,
        };
        let up = Event::MouseUp {
            pos,
            button: MouseButton::Left,
        };
        assert!(block.handle_event(state, &down));
        block.handle_event(state, &up);
        block.update(state);
    }

    #[test]
    fn new_block_button_adds_a_block() {
        let mut block = Block::new();
        let mut state = new_state();
        click(&mut block, &mut state, NEW_BLOCK);
        assert_eq!(state.song().patterns.len(), 2);
    }

    #[test]
    fn delete_this_button_removes_the_active_block() {
        let mut block = Block::new();
        let mut state = new_state();
        click(&mut block, &mut state, NEW_BLOCK);
        click(&mut block, &mut state, DELETE_THIS);
        assert_eq!(state.song().patterns.len(), 1);
    }

    #[test]
    fn track_count_buttons_are_disabled() {
        // 5-8 channel mode isn't supported yet
        let block = Block::new();
        assert!(block.track_counts.iter().all(|button| !button.is_enabled()));
    }

    #[test]
    fn ins_track_and_del_track_change_track_count() {
        let mut block = Block::new();
        let mut state = new_state();
        let before = state.active_pattern().unwrap().track_count();
        click(&mut block, &mut state, INS_TRK);
        assert_eq!(state.active_pattern().unwrap().track_count(), before);
        click(&mut block, &mut state, DEL_TRK);
        assert_eq!(state.active_pattern().unwrap().track_count(), before);
    }

    #[test]
    fn lines_steppers_change_line_count() {
        let mut block = Block::new();
        let mut state = new_state();
        let before = state.active_pattern().unwrap().line_count();
        click(&mut block, &mut state, LINES_INC_10);
        assert_eq!(state.active_pattern().unwrap().line_count(), before + 10);
        click(&mut block, &mut state, LINES_DEC);
        assert_eq!(state.active_pattern().unwrap().line_count(), before + 9);
    }

    #[test]
    fn expand_and_shrink_use_the_factor_box() {
        let mut block = Block::new();
        let mut state = new_state();
        block.expand_factor.set_text("3");
        let before = state.active_pattern().unwrap().line_count();
        click(&mut block, &mut state, EXPAND);
        assert_eq!(state.active_pattern().unwrap().line_count(), before * 3);
        click(&mut block, &mut state, SHRINK);
        assert_eq!(state.active_pattern().unwrap().line_count(), before);
    }

    #[test]
    fn expand_past_max_lines_is_declined() {
        let mut block = Block::new();
        let mut state = new_state();
        state
            .active_pattern_mut()
            .unwrap()
            .set_line_count(MAX_LINES);
        click(&mut block, &mut state, EXPAND);
        assert_eq!(state.active_pattern().unwrap().line_count(), MAX_LINES);
        assert_eq!(state.notification.as_deref(), Some("Too many lines"));
    }

    #[test]
    fn ins_line_and_del_line_change_line_count() {
        let mut block = Block::new();
        let mut state = new_state();
        let before = state.active_pattern().unwrap().line_count();
        click(&mut block, &mut state, INS_LINE);
        assert_eq!(state.active_pattern().unwrap().line_count(), before + 1);
        click(&mut block, &mut state, DEL_LINE);
        assert_eq!(state.active_pattern().unwrap().line_count(), before);
    }

    #[test]
    fn highlight_buttons_set_and_clear_spacing() {
        let mut block = Block::new();
        let mut state = new_state();
        click(&mut block, &mut state, HIGHLIGHT[1]); // "2"
        let pattern = state.active_pattern().unwrap();
        assert!(pattern.lines[0].highlighted);
        assert!(!pattern.lines[1].highlighted);
        assert!(pattern.lines[2].highlighted);
        click(&mut block, &mut state, HIGHLIGHT[6]); // "C"
        assert!(
            state
                .active_pattern()
                .unwrap()
                .lines
                .iter()
                .all(|l| !l.highlighted)
        );
    }

    #[test]
    fn split_then_join_via_buttons() {
        let mut block = Block::new();
        let mut state = new_state();
        state.cursor.line = 32;
        click(&mut block, &mut state, SPLIT);
        assert_eq!(state.song().patterns.len(), 2);
        click(&mut block, &mut state, JOIN);
        assert_eq!(state.song().patterns.len(), 1);
    }
}
