use ggez::graphics::Color;
use taffy::{
    Size, Style, TaffyTree,
    prelude::{auto, length, percent},
};

use crate::framework::{
    widget::{Border, Component, DefaultStyle, Widget},
    widgets::container::Container,
};

pub struct StatusBar {
    container: Container,
}

impl StatusBar {
    pub fn new(taffy: &mut TaffyTree) -> Self {

        let border = Color::BLACK;

        let fill = Color::from_rgb(211, 211, 211);

        let left_container = Container::new(taffy, vec![])
            .with_style(
                taffy,
                Style {
                    size: Size {
                        width: percent(1.0),
                        height: auto(),
                    },
                    ..Container::default_style()
                },
            )
            .with_colors(border, fill)
            .with_border(border, 2.0)
            .boxed();

        let container = Container::new(taffy, vec![left_container])
            .with_style(taffy, Self::default_style())
            .with_border(border, 2.0);

        return Self {
            container,
        };
    }
}

impl Component for StatusBar {
    fn container(&self) -> &Container {

        &self.container
    }

    fn container_mut(&mut self) -> &mut Container {

        &mut self.container
    }
}

impl DefaultStyle for StatusBar {
    fn default_style() -> Style
    where
        Self: Sized,
    {

        Style {
            size: Size {
                width: percent(1.0),
                height: length(30.0),
            },
            ..Container::default_style()
        }
    }
}
