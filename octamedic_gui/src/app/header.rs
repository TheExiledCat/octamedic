use ggez::graphics::Color;
use taffy::{
    NodeId, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{ui::Ui, widgets::panel::Panel};

pub fn build(ui: &mut Ui, parent: NodeId) -> NodeId {

    let border = Color::BLACK;

    let fill = Color::from_rgb(211, 211, 211);

    let header = ui.add_styled(parent, Panel::new().with_border(border, 2.0), |style| {

        style.size.height = length(130.0_f32);
    });

    let side = |width: f32| {

        move |style: &mut Style| {

            style.size = Size {
                width: percent(width),
                height: auto(),
            };
        }
    };

    ui.add_styled(
        header,
        Panel::new().with_background(fill).with_border(border, 2.0),
        side(0.7),
    );

    ui.add_styled(header, Panel::new().with_background(fill), side(0.3));

    return header;
}
