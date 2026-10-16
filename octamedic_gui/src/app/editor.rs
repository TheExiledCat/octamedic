use ggez::graphics::Color;
use taffy::{
    FlexDirection, NodeId, Rect, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{
    color::FromHex,
    ui::Ui,
    widgets::{label::Label, panel::Panel},
};

const COLUMNS: usize = 9;

const ROWS: usize = 15;

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {

    let editor = ui.add(
        parent,
        Style {
            size: Size {
                width: percent(1.0_f32),
                height: auto(),
            },
            margin: Rect {
                left: length(2.0_f32),
                ..Rect::zero()
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Panel::default_style()
        },
        Panel::new().with_background(Color::hex("000")),
    );

    let row = ui.add(
        editor,
        Style {
            size: Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            },
            gap: Size::zero(),
            ..Panel::default_style()
        },
        Panel::new().with_background(Color::BLACK),
    );

    for column in 0..COLUMNS {

        let column_node = ui.add(
            row,
            Style {
                size: Size {
                    width: auto(),
                    height: percent(1.0_f32),
                },
                margin: Rect {
                    left: length(1.0_f32),
                    ..Rect::zero()
                },
                flex_direction: FlexDirection::Column,
                ..Panel::default_style()
            },
            Panel::new().with_border(Color::hex("E4A0B7"), 1.0),
        );

        for line in 0..ROWS {

            let text = if column == 0 {

                format!("{:03} ", line)
            } else {

                "--- 00000".into()
            };

            ui.add(column_node, Label::default_style(), Label::new(text, 19.0));
        }
    }

    return editor;
}
