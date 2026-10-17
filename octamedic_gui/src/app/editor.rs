use std::time::Instant;

use ggez::graphics::{Canvas, Color, Rect as ScreenRect};
use taffy::{
    FlexDirection, NodeId, Rect, Size, Style,
    prelude::{auto, length, percent},
};

use crate::{
    app::state::AppState,
    framework::{
        color::FromHex,
        ui::Ui,
        widget::{DrawState, Widget},
        widgets::{
            label::Label,
            panel::{Borders, Panel, fill_rect},
        },
    },
};

const COLUMNS: usize = 9;

// odd, so there is a middle row for the selected line
const ROWS: usize = 15;

const CENTER_ROW: usize = ROWS / 2;

pub struct BlockEditor {
    selected_row: usize,
    selected_track: u8,
    visible_rows: Vec<Vec<NodeId>>, // visible row -> its labels, in column order
    line_count: usize,
    displayed: Option<(usize, usize)>, // the selected row and line count the labels show
    highlight: Option<ScreenRect>,
    last_instant: Instant,
}

impl BlockEditor {
    pub fn new() -> Self {

        return Self {
            selected_row: 0,
            selected_track: 0,
            visible_rows: Vec::new(),
            line_count: 0,
            displayed: None,
            highlight: None,
            last_instant: Instant::now(),
        };
    }

    fn show_lines_around_selection(&self, ui: &mut Ui) {

        for (row, labels) in self.visible_rows.iter().enumerate() {

            let line = self.selected_row as isize + row as isize - CENTER_ROW as isize;

            let in_block = (0..self.line_count as isize).contains(&line);

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

    fn update(&mut self, ui: &mut Ui, state: &mut AppState, node: NodeId) {

        self.line_count = state
            .active_pattern()
            .map_or(0, |pattern| pattern.line_count());

        if state.is_playing() {

            self.selected_row = state.engine.status().row();
        } else if self.last_instant.elapsed().as_secs() >= 1 {

            self.last_instant = Instant::now();

            self.selected_row = (self.selected_row + 1) % self.line_count.max(1);
        }

        let shown = (self.selected_row, self.line_count);

        if self.displayed != Some(shown) {

            self.show_lines_around_selection(ui);

            self.displayed = Some(shown);
        }

        self.highlight = self.highlight_rect(ui, node);
    }

    fn draw(&self, canvas: &mut Canvas, draw: &DrawState, _state: &AppState) {

        fill_rect(canvas, draw.rect, Color::hex("000"));

        if let Some(highlight) = self.highlight {

            fill_rect(canvas, highlight, Color::hex("3B3B6D"));
        }
    }
}

#[cfg(test)]

mod tests {

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    // the default project's block has 64 lines
    const LINE_COUNT: usize = 64;

    fn mounted(selected_row: usize) -> (Ui, NodeId) {

        let mut ui = Ui::new(Panel::new());

        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());

        let mut editor = BlockEditor::new();

        editor.selected_row = selected_row;

        let root = ui.root();

        let node = ui.add(root, editor);

        ui.update(&mut state);

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

        let (ui, editor) = mounted(LINE_COUNT - 1);

        assert!(!row(&ui, editor, CENTER_ROW + 1).1);
    }
}
