use std::time::Instant;

use ggez::graphics::{Canvas, Color, Rect as ScreenRect};
use taffy::{
    FlexDirection, NodeId, Rect, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{
    color::FromHex,
    ui::Ui,
    widget::{DrawState, Widget},
    widgets::{
        label::Label,
        panel::{Borders, Panel, fill_rect},
    },
};

const COLUMNS: usize = 9;

// odd, so there is a middle row for the selected line
const ROWS: usize = 15;

const CENTER_ROW: usize = ROWS / 2;

// a stand-in block length until the editor shows real blocks
const LINE_COUNT: usize = 64;

pub struct BlockEditor {
    selected_row: u8,
    selected_track: u8,
    visible_rows: Vec<Vec<NodeId>>, // visible row -> its labels, in column order
    displayed_row: Option<u8>,
    highlight: Option<ScreenRect>,
    last_instant: Instant,
}

impl BlockEditor {
    pub fn new() -> Self {

        return Self {
            selected_row: 0,
            selected_track: 0,
            visible_rows: Vec::new(),
            displayed_row: None,
            highlight: None,
            last_instant: Instant::now(),
        };
    }

    fn show_lines_around_selection(&self, ui: &mut Ui) {

        for (row, labels) in self.visible_rows.iter().enumerate() {

            let line = self.selected_row as isize + row as isize - CENTER_ROW as isize;

            let in_block = (0..LINE_COUNT as isize).contains(&line);

            for (column, &label) in labels.iter().enumerate() {

                let Some(label) = ui.get_mut::<Label>(label) else {

                    continue;
                };

                if column == 0 && in_block {

                    label.set_text(format!("{:03} ", line));
                }

                label.set_color(if in_block {

                    Color::WHITE
                } else {

                    Color::from_rgba(0, 0, 0, 0)
                });
            }
        }
    }

    fn highlight_rect(&self, ui: &Ui, editor: NodeId) -> Option<ScreenRect> {

        let editor = ui.screen_rect(editor)?;

        let center = ui.screen_rect(*self.visible_rows.get(CENTER_ROW)?.first()?)?;

        return Some(ScreenRect::new(editor.x, center.y, editor.w, center.h));
    }
}

impl Widget for BlockEditor {
    fn style(&self) -> Style {

        return Style {
            size: Size {
                width: percent(1.0_f32),
                height: auto(),
            },
            margin: Rect {
                left: length(2.0_f32),
                ..Rect::zero()
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Panel::new().style()
        };
    }

    fn mount(&mut self, ui: &mut Ui, editor: NodeId) {

        // no background of its own, so the highlight the editor draws behind it shows through
        let row = ui.add_styled(editor, Panel::new(), |style| {

            style.size = Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            };
        });

        self.visible_rows = vec![Vec::with_capacity(COLUMNS); ROWS];

        for column in 0..COLUMNS {

            // a line between columns only, so not after the last one
            let borders = if column + 1 < COLUMNS {

                Borders::RIGHT
            } else {

                Borders::NONE
            };

            let column_node = ui.add_styled(
                row,
                Panel::new().with_borders(borders, Color::hex("E4A0B7"), 1.0),
                |style| {

                    style.size = Size {
                        width: auto(),
                        height: percent(1.0_f32),
                    };

                    style.margin.left = length(1.0_f32);

                    style.flex_direction = FlexDirection::Column;
                },
            );

            for visible_row in &mut self.visible_rows {

                let text = if column == 0 { "000 " } else { "--- 00000" };

                visible_row.push(ui.add(column_node, Label::new(text, 19.0)));
            }
        }
    }

    fn update(&mut self, ui: &mut Ui, node: NodeId) {

        if self.last_instant.elapsed().as_secs() >= 1 {

            self.last_instant = Instant::now();

            self.selected_row = ((self.selected_row as usize + 1) % LINE_COUNT) as u8;
        }

        if self.displayed_row != Some(self.selected_row) {

            self.show_lines_around_selection(ui);

            self.displayed_row = Some(self.selected_row);
        }

        self.highlight = self.highlight_rect(ui, node);
    }

    fn draw(&self, canvas: &mut Canvas, state: &DrawState) {

        fill_rect(canvas, state.rect, Color::hex("000"));

        if let Some(highlight) = self.highlight {

            fill_rect(canvas, highlight, Color::hex("3B3B6D"));
        }
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    fn mounted(selected_row: u8) -> (Ui, NodeId) {

        let mut ui = Ui::new(Panel::new());

        let mut editor = BlockEditor::new();

        editor.selected_row = selected_row;

        let root = ui.root();

        let node = ui.add(root, editor);

        ui.update();

        return (ui, node);
    }

    fn row(ui: &Ui, editor: NodeId, row: usize) -> (String, bool) {

        let label = ui.get::<BlockEditor>(editor).unwrap().visible_rows[row][0];

        let label = ui.get::<Label>(label).unwrap();

        return (label.text().to_string(), label.color().a > 0.0);
    }

    #[test]

    fn the_selected_line_is_in_the_middle_row() {

        let (ui, editor) = mounted(20);

        assert_eq!(row(&ui, editor, CENTER_ROW), ("020 ".to_string(), true));

        assert_eq!(row(&ui, editor, 0), ("013 ".to_string(), true));

        assert_eq!(row(&ui, editor, ROWS - 1), ("027 ".to_string(), true));
    }

    #[test]

    fn rows_outside_the_block_are_hidden() {

        let (ui, editor) = mounted(0);

        assert!(!row(&ui, editor, CENTER_ROW - 1).1);

        assert_eq!(row(&ui, editor, CENTER_ROW), ("000 ".to_string(), true));

        let (ui, editor) = mounted(LINE_COUNT as u8 - 1);

        assert!(!row(&ui, editor, CENTER_ROW + 1).1);
    }
}
