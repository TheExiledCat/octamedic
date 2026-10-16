use ggez::graphics::Color;
use taffy::{
    NodeId, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{ui::Ui, widgets::panel::Panel};

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {

    let border = Color::BLACK;

    let fill = Color::from_rgb(211, 211, 211);

    let header = ui.add(
        parent,
        Style {
            size: Size {
                width: percent(1.0_f32),
                height: length(130.0_f32),
            },
            ..Panel::default_style()
        },
        Panel::new().with_border(border, 2.0),
    );

    let side = |width: f32| Style {
        size: Size {
            width: percent(width),
            height: auto(),
        },
        ..Panel::default_style()
    };

    ui.add(
        header,
        side(0.7),
        Panel::new().with_background(fill).with_border(border, 2.0),
    );

    ui.add(header, side(0.3), Panel::new().with_background(fill));

    return header;
}
