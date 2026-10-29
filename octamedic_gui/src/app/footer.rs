use ggez::graphics::Color;
use taffy::{NodeId, Size, prelude::percent};

use crate::framework::{ui::Ui, widgets::panel::Panel};

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {
    let border = Color::BLACK;
    let fill = Color::from_rgb(211, 211, 211);
    let footer = ui.add_styled(
        parent,
        Panel::new().with_background(fill).with_border(border, 2.0),
        |style| style.size.height = percent(0.25_f32),
    );
    ui.add_styled(
        footer,
        Panel::new().with_background(fill).with_border(border, 2.0),
        |style| {
            style.size = Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            };
        },
    );
    return footer;
}
