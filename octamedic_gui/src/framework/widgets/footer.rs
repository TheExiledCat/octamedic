use ggez::graphics::Color;
use taffy::{Size, Style, TaffyTree, prelude::percent};

use crate::framework::{
    widget::{Border, Component, DefaultStyle, Widget},
    widgets::container::Container,
};

pub struct Footer {
    container: Container,
}

impl Footer {
    pub fn new(taffy: &mut TaffyTree) -> Self {

        let border = Color::BLACK;

        let fill = Color::from_rgb(211, 211, 211);

        let left_container = Container::new(taffy, vec![])
            .with_style(
                taffy,
                Style {
                    size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    ..Container::default_style()
                },
            )
            .with_colors(border, fill)
            .with_border(border, 2.0)
            .boxed();

        let container = Container::new(taffy, vec![left_container])
            .with_style(taffy, Self::default_style())
            .with_colors(border, fill)
            .with_border(border, 2.0);

        return Self {
            container,
        };
    }
}

impl Component for Footer {
    fn container(&self) -> &Container {

        &self.container
    }

    fn container_mut(&mut self) -> &mut Container {

        &mut self.container
    }
}

impl DefaultStyle for Footer {
    fn default_style() -> Style
    where
        Self: Sized,
    {

        Style {
            size: Size {
                width: percent(1.0),
                height: percent(0.25),
            },
            ..Container::default_style()
        }
    }
}
