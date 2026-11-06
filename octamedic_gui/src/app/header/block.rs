use rustuition::{
    font::{Font, Glyph},
    gadget::Gadget,
    gadgets::button::{ContentArea, DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL},
    painter::{Painter, Pen, inside_frame},
    rect::Rect,
};

use crate::{
    app::{fonts, header::finished_edit, pens, state::AppState},
    framework::{
        event::Event,
        widgets::{Button, Textbox, ToggleGroup},
    },
};

// from the panel's top left, measured from OctaMED. The rows are 11px with a 1px gap above the
// third
// the labels' tops, a row for each row of buttons
const TOP_ROW: i32 = 3;
const SECOND_ROW: i32 = 14;
const TRK_LABEL: Words = &[(2, "TRK"), (26, ":")];
const CUT: Rect = Rect::new(30, 0, 48, 11);
const COPY: Rect = Rect::new(78, 0, 48, 11);
const PASTE: Rect = Rect::new(126, 0, 48, 11);
const SWAP: Rect = Rect::new(174, 0, 48, 11);
const INS_TRK: Rect = Rect::new(224, 0, 54, 11);
const TRKS_LABEL: Words = &[(280, "TRKS"), (312, ":")];
const TRK_COUNT_4: Rect = Rect::new(316, 0, 33, 11);
const TRK_COUNT_8: Rect = Rect::new(349, 0, 33, 11);
const TRK_COUNT_12: Rect = Rect::new(382, 0, 33, 11);
const TRK_COUNT_16: Rect = Rect::new(415, 0, 33, 11);

const BLK_LABEL: Words = &[(3, "BLK"), (26, ":")];
const CUT_BLOCK: Rect = Rect::new(30, 11, 48, 11);
const COPY_BLOCK: Rect = Rect::new(78, 11, 48, 11);
const PASTE_BLOCK: Rect = Rect::new(126, 11, 48, 11);
const SWAP_BLOCK: Rect = Rect::new(174, 11, 48, 11);
const DEL_TRK: Rect = Rect::new(224, 11, 54, 11);
const LINES_LABEL: Words = &[(280, "LINES"), (315, ":")];
const LINES_BOX: Rect = Rect::new(324, 11, 44, 11);
const LINES_DEC: Rect = Rect::new(368, 11, 20, 11);
const LINES_INC: Rect = Rect::new(388, 11, 20, 11);
const LINES_DEC_10: Rect = Rect::new(408, 11, 20, 11);
const LINES_INC_10: Rect = Rect::new(428, 11, 20, 11);

const EXPAND: Rect = Rect::new(0, 23, 78, 11);
const EXPAND_FACTOR: Rect = Rect::new(78, 23, 24, 11);
const SHRINK: Rect = Rect::new(102, 23, 78, 11);
const INS_LINE: Rect = Rect::new(180, 23, 50, 11);
const DEL_LINE: Rect = Rect::new(230, 23, 50, 11);
const DEL_BUFF: Rect = Rect::new(280, 23, 70, 11);
const HIGHLIGHT: [Rect; 7] = [
    Rect::new(350, 23, 14, 11),
    Rect::new(364, 23, 14, 11),
    Rect::new(378, 23, 14, 11),
    Rect::new(392, 23, 14, 11),
    Rect::new(406, 23, 14, 11),
    Rect::new(420, 23, 14, 11),
    Rect::new(434, 23, 14, 11),
];
// the spacing each HIGHLIGHT button sets, C (last) clears it instead
const HIGHLIGHT_SPACING: [usize; 6] = [1, 2, 3, 4, 8, 16];

const NEW_BLOCK: Rect = Rect::new(0, 34, 65, 11);
const NEW_BLOCK_DEF: Rect = Rect::new(65, 34, 25, 11);
const NEW_HERE: Rect = Rect::new(90, 34, 65, 11);
const NEW_HERE_DEF: Rect = Rect::new(155, 34, 25, 11);
const DELETE_LAST: Rect = Rect::new(180, 34, 90, 11);
const DELETE_THIS: Rect = Rect::new(270, 34, 90, 11);
const SPLIT: Rect = Rect::new(360, 34, 44, 11);
const JOIN: Rect = Rect::new(404, 34, 44, 11);

const TRACK_COUNTS: [usize; 4] = [4, 8, 12, 16];

// where the boxes' numbers start past their frames
const LINES_PADDING: i32 = 0;
const EXPAND_FACTOR_PADDING: i32 = 2;

pub struct Block {
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
        let label = |words: Words| Button::new(placed(fonts::LABEL, words));
        let narrow = |words: Words| Button::new(placed(fonts::NARROW_LABEL, words));
        let number_box = |padding| {
            Textbox::new()
                .font(fonts::INTERFACE)
                .pens(Pen::BACKGROUND, Pen::SHINE, Pen::FILL, pens::RED)
                .padding(padding)
                .select_on_focus()
        };
        return Self {
            cut_track: label(&[(13, "CUT")]),
            copy_track: label(&[(10, "COPY")]),
            paste_track: label(&[(6, "PASTE")]),
            swap_track: label(&[(9, "SWAP")]),
            cut_block: label(&[(13, "CUT")]),
            copy_block: label(&[(10, "COPY")]),
            paste_block: label(&[(6, "PASTE")]),
            swap_block: label(&[(9, "SWAP")]),
            del_buff: label(&[(7, "DEL"), (35, "BUFF")]),

            ins_track: narrow(&[(6, "INS.TRK")]),
            del_track: narrow(&[(5, "DEL.TRK")]),
            // 5-8 channel mode (8/12/16 tracks) isn't supported yet, so there's nothing to pick
            track_counts: TRACK_COUNT_DIGITS.map(|digits| unhooked(Button::new(picture(digits)))),

            lines_box: number_box(LINES_PADDING),
            lines_box_was_focused: false,
            lines_dec: step_button(&ARROW_LEFT),
            lines_inc: step_button(&ARROW_RIGHT),
            lines_dec_10: step_button(&DOUBLE_ARROW_LEFT),
            lines_inc_10: step_button(&DOUBLE_ARROW_RIGHT),

            expand: label(&[(15, "EXPAND")]),
            expand_factor: {
                let mut field = number_box(EXPAND_FACTOR_PADDING);
                field.set_text("2");
                field
            },
            shrink: label(&[(20, "SHRINK")]),
            ins_line: label(&[(8, "INS."), (35, "L.")]),
            del_line: label(&[(7, "DEL."), (35, "L.")]),
            highlight: HIGHLIGHT_LABELS.map(label),

            new_block: narrow(&[(4, "NEW"), (29, "BLOCK")]),
            new_block_def: narrow(&[(4, "DEF")]),
            new_here: narrow(&[(7, "NEW"), (32, "HERE")]),
            new_here_def: narrow(&[(4, "DEF")]),
            delete_last: label(&[(6, "DELETE"), (56, "LAST")]),
            delete_this: label(&[(8, "DELETE"), (58, "THIS")]),
            split: label(&[(6, "SPLIT")]),
            join: label(&[(8, "JOIN")]),
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

const HIGHLIGHT_LABELS: [Words; 7] = [
    &[(5, "1")],
    &[(5, "2")],
    &[(5, "3")],
    &[(5, "4")],
    &[(5, "8")],
    &[(3, "1"), (7, "6")],
    &[(4, "C")],
];

// OctaMED drew this panel's labels by hand rather than centring them: each word with where its
// first pixel is from the button's left
type Words = &'static [(i32, &'static str)];

// the labels' top from the button's
const LABEL_Y: i32 = 3;

fn placed(font: Font, words: Words) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let [left, top] = button_origin(area);
        draw_words(painter, font, [left, top + LABEL_Y], words, pen);
    };
}

fn draw_words(painter: &mut Painter, font: Font, [left, top]: [i32; 2], words: Words, pen: Pen) {
    for &(x, word) in words {
        let bearing = word
            .chars()
            .next()
            .and_then(|c| font.glyph(c))
            .map_or(0, blank_columns);
        painter.text_in_font(left + x - bearing, top, word, pen, font);
    }
}

// a glyph's empty columns on its left
fn blank_columns(glyph: &Glyph) -> i32 {
    let lit = glyph.lines.iter().filter(|&&line| line != 0);
    return lit
        .map(|line| line.leading_zeros() as i32)
        .min()
        .unwrap_or(0);
}

// a picture drawn a pixel per '#', from where its top left is in the button
struct Picture {
    at: [i32; 2],
    rows: &'static [&'static str],
}

fn picture(picture: &'static Picture) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let [left, top] = button_origin(area);
        let [x, y] = picture.at;
        for (row, line) in picture.rows.iter().enumerate() {
            for (column, pixel) in line.bytes().enumerate() {
                if pixel == b'#' {
                    let pixel = Rect::new(left + x + column as i32, top + y + row as i32, 1, 1);
                    painter.fill(pixel, pen);
                }
            }
        }
    };
}

// the button's own top left, outside its frame
fn button_origin(area: ContentArea) -> [i32; 2] {
    let frame = inside_frame(Rect::new(0, 0, 0, 0), 1);
    return [area.inside.x - frame.x, area.inside.y - frame.y];
}

const ARROW_LEFT: Picture = Picture {
    at: [6, 2],
    rows: &[
        "......##", "....####", "..######", "########", "..######", "....####", "......##",
    ],
};
const ARROW_RIGHT: Picture = Picture {
    at: [6, 2],
    rows: &[
        "##......", "####....", "######..", "########", "######..", "####....", "##......",
    ],
};
const DOUBLE_ARROW_LEFT: Picture = Picture {
    at: [6, 2],
    rows: &[
        "...#...#", "..##..##", ".###.###", "########", ".###.###", "..##..##", "...#...#",
    ],
};
const DOUBLE_ARROW_RIGHT: Picture = Picture {
    at: [6, 2],
    rows: &[
        "#...#...", "##..##..", "###.###.", "########", "###.###.", "##..##..", "#...#...",
    ],
};

// TRKS: has its own big digits
const TRACK_COUNT_DIGITS: [&Picture; 4] = [
    &Picture {
        at: [9, 2],
        rows: &[
            "....###...###..",
            "...###....###..",
            "..###.....###..",
            ".###......###..",
            "###############",
            "..........###..",
            "..........###..",
        ],
    },
    &Picture {
        at: [11, 2],
        rows: &[
            ".###########.",
            "###.......###",
            "###.......###",
            ".###########.",
            "###.......###",
            "###.......###",
            ".###########.",
        ],
    },
    &Picture {
        at: [7, 2],
        rows: &[
            "..###....#########.",
            ".####...###.....###",
            "#####...........###",
            "..###.........###..",
            "..###.......###....",
            "..###.....###......",
            "#######..##########",
        ],
    },
    &Picture {
        at: [7, 2],
        rows: &[
            "..###....#########.",
            ".####...###.....###",
            "#####...###........",
            "..###...##########.",
            "..###...###.....###",
            "..###...###.....###",
            "#######..#########.",
        ],
    },
];

fn step_button(arrow: &'static Picture) -> Button {
    return Button::new(picture(arrow)).repeat(DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL);
}

impl Gadget<AppState, Event> for Block {
    fn update(&mut self, state: &mut AppState) {
        let clipboard: [(&mut Button, fn(&mut AppState)); 9] = [
            (&mut self.cut_track, AppState::cut_track),
            (&mut self.copy_track, AppState::copy_track),
            (&mut self.paste_track, AppState::paste_track),
            (&mut self.swap_track, AppState::swap_track),
            (&mut self.cut_block, AppState::cut_block),
            (&mut self.copy_block, AppState::copy_block),
            (&mut self.paste_block, AppState::paste_block),
            (&mut self.swap_block, AppState::swap_block),
            (&mut self.del_buff, AppState::delete_buffer),
        ];
        for (button, action) in clipboard {
            if button.clicked().is_some() {
                action(state);
            }
        }
        if self.ins_track.clicked().is_some() {
            state.insert_track();
        }
        if self.del_track.clicked().is_some() {
            state.delete_track();
        }
        let active_tracks = state.active_pattern().map_or(0, |p| p.track_count());
        let mut track_count_group =
            ToggleGroup::new(self.track_counts.iter_mut().zip(TRACK_COUNTS));
        if let Some((count, _)) = track_count_group.take_click() {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_track_count(count);
            }
            state.clamp_cursor_to_active_pattern();
        }
        track_count_group.sync(Some(active_tracks));

        // committed before the box is refreshed, or the refresh would overwrite what was typed
        let typed_lines = finished_edit(&self.lines_box, &mut self.lines_box_was_focused)
            .and_then(|text| text.trim().parse::<usize>().ok());
        if let Some(lines) = typed_lines {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_line_count(lines);
            }
            state.clamp_cursor_to_active_pattern();
        }
        let step_lines = |state: &mut AppState, delta: isize| {
            let lines = state.active_pattern().map_or(1, |p| p.line_count()) as isize;
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.set_line_count((lines + delta).max(1) as usize);
            }
            state.clamp_cursor_to_active_pattern();
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
        if !self.lines_box.is_focused() {
            let lines = state.active_pattern().map_or(0, |p| p.line_count());
            self.lines_box.set_text(lines.to_string());
        }

        let expand_factor = self.expand_factor.text().trim().parse().unwrap_or(2).max(1);
        if self.expand.clicked().is_some() {
            let expanded = state
                .active_pattern_mut()
                .is_some_and(|pattern| pattern.expand(expand_factor));
            if !expanded {
                state.notify("Too many lines");
            }
        }
        if self.shrink.clicked().is_some() {
            if let Some(pattern) = state.active_pattern_mut() {
                pattern.shrink(expand_factor);
            }
            state.clamp_cursor_to_active_pattern();
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
            state.clamp_cursor_to_active_pattern();
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
        // both see every click, it's how a text box finds out it lost focus
        let handled = self.lines_box.handle_event(state, event)
            | self.expand_factor.handle_event(state, event);
        return handled
            || self
                .buttons()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        draw_label(painter, area, TOP_ROW, TRK_LABEL);
        self.cut_track.render(state, CUT.within(area), painter);
        self.copy_track.render(state, COPY.within(area), painter);
        self.paste_track.render(state, PASTE.within(area), painter);
        self.swap_track.render(state, SWAP.within(area), painter);
        self.ins_track.render(state, INS_TRK.within(area), painter);
        // OctaMED's T here is missing its first column
        let clip = Rect::new(area.x + TRKS_LABEL[0].0 + 1, area.y, area.w, area.h);
        painter.clipped(clip, |painter| {
            draw_label(painter, area, TOP_ROW, TRKS_LABEL);
        });
        let track_count_rects = [TRK_COUNT_4, TRK_COUNT_8, TRK_COUNT_12, TRK_COUNT_16];
        for (button, rect) in self.track_counts.iter_mut().zip(track_count_rects) {
            button.render(state, rect.within(area), painter);
        }

        draw_label(painter, area, SECOND_ROW, BLK_LABEL);
        self.cut_block
            .render(state, CUT_BLOCK.within(area), painter);
        self.copy_block
            .render(state, COPY_BLOCK.within(area), painter);
        self.paste_block
            .render(state, PASTE_BLOCK.within(area), painter);
        self.swap_block
            .render(state, SWAP_BLOCK.within(area), painter);
        self.del_track.render(state, DEL_TRK.within(area), painter);
        draw_label(painter, area, SECOND_ROW, LINES_LABEL);
        render_number_box(&mut self.lines_box, state, LINES_BOX.within(area), painter);
        self.lines_dec
            .render(state, LINES_DEC.within(area), painter);
        self.lines_inc
            .render(state, LINES_INC.within(area), painter);
        self.lines_dec_10
            .render(state, LINES_DEC_10.within(area), painter);
        self.lines_inc_10
            .render(state, LINES_INC_10.within(area), painter);

        self.expand.render(state, EXPAND.within(area), painter);
        render_number_box(
            &mut self.expand_factor,
            state,
            EXPAND_FACTOR.within(area),
            painter,
        );
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

fn render_number_box(textbox: &mut Textbox, state: &AppState, area: Rect, painter: &mut Painter) {
    textbox.render(state, area, painter);
    painter.frame_inverted(area, 1);
}

fn draw_label(painter: &mut Painter, area: Rect, row: i32, words: Words) {
    draw_words(
        painter,
        fonts::LABEL,
        [area.x, area.y + row],
        words,
        Pen::BACKGROUND,
    );
}

fn set_highlight_spacing(state: &mut AppState, spacing: Option<usize>) {
    if let Some(pattern) = state.active_pattern_mut() {
        for (index, line) in pattern.lines.iter_mut().enumerate() {
            line.highlighted = spacing.is_some_and(|spacing| index % spacing == 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{
        data::{pattern::MAX_LINES, project::OctamedicProject},
        engine::handle::EngineHandle,
    };
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
        .chain(HIGHLIGHT.into_iter().map(|rect| ("HIGHLIGHT", rect)))
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
    fn track_copy_and_paste_buttons() {
        let mut block = Block::new();
        let mut state = new_state();
        state.active_pattern_mut().unwrap().lines[0].tracks[0]
            .note
            .value = 5;
        click(&mut block, &mut state, COPY);
        state.cursor.track = 1;
        click(&mut block, &mut state, PASTE);
        assert_eq!(
            state.active_pattern().unwrap().lines[0].tracks[1]
                .note
                .value,
            5
        );
        click(&mut block, &mut state, DEL_BUFF);
        assert!(state.track_buffer.is_none());
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
    fn typing_into_the_lines_box_sets_the_line_count() {
        let mut block = Block::new();
        let mut state = new_state();
        block.lines_box.set_focused(true);
        block.update(&mut state);
        block.lines_box.set_text("100");
        block.lines_box.set_focused(false);
        block.update(&mut state);
        assert_eq!(state.active_pattern().unwrap().line_count(), 100);
        assert_eq!(block.lines_box.text(), "100");
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
