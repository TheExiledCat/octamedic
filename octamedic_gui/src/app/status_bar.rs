use ggez::graphics::Color;
use taffy::{
    NodeId, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{ui::Ui, widgets::panel::Panel};

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {

    let border = Color::BLACK;

    let status_bar = ui.add(
        parent,
        Style {
            size: Size {
                width: percent(1.0_f32),
                height: length(30.0_f32),
            },
            ..Panel::default_style()
        },
        Panel::new().with_border(border, 2.0),
    );

    ui.add(
        status_bar,
        Style {
            size: Size {
                width: percent(1.0_f32),
                height: auto(),
            },
            ..Panel::default_style()
        },
        Panel::new()
            .with_background(Color::from_rgb(211, 211, 211))
            .with_border(border, 2.0),
    );

    return status_bar;
}
