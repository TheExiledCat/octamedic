use ggez::graphics::Color;
use taffy::{
    NodeId, Size,
    prelude::{auto, length, percent},
};

use crate::framework::{ui::Ui, widgets::panel::Panel};

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {
    let border = Color::BLACK;
    let status_bar = ui.add_styled(parent, Panel::new().with_border(border, 2.0), |style| {
        style.size.height = length(30.0_f32);
    });
    ui.add_styled(
        status_bar,
        Panel::new()
            .with_background(Color::from_rgb(211, 211, 211))
            .with_border(border, 2.0),
        |style| {
            style.size = Size {
                width: percent(1.0_f32),
                height: auto(),
            };
        },
    );
    return status_bar;
}
