use crate::framework::widget::{Border, Component, DefaultStyle, Widget};
use crate::framework::widgets::container::Container;
use ggez::graphics::Color;
use taffy::prelude::{auto, length, percent};
use taffy::{Size, Style, TaffyTree};

pub struct HeaderBar {
    container: Container,
}
impl HeaderBar {
    pub fn new(taffy: &mut TaffyTree) -> Self {
        let border = Color::BLACK;
        let fill = Color::from_rgb(211, 211, 211);
        let left_container = Container::new(taffy, vec![])
            .with_style(
                taffy,
                Style {
                    size: Size {
                        width: percent(0.7),
                        height: auto(),
                    },
                    ..Container::default_style()
                },
            )
            .with_colors(border, fill)
            .with_border(border, 2.0)
            .boxed();
        let right_container = Container::new(taffy, vec![])
            .with_style(
                taffy,
                Style {
                    size: Size {
                        width: percent(0.3),
                        height: auto(),
                    },
                    ..Container::default_style()
                },
            )
            .with_colors(border, fill)
            .boxed();
        let container = Container::new(taffy, vec![left_container, right_container])
            .with_style(taffy, Self::default_style())
            .with_border(border, 2.0);
        return Self { container };
    }
}

impl Component for HeaderBar {
    fn container(&self) -> &Container {
        &self.container
    }

    fn container_mut(&mut self) -> &mut Container {
        &mut self.container
    }
}
impl DefaultStyle for HeaderBar {
    fn default_style() -> Style
    where
        Self: Sized,
    {
        Style {
            size: Size {
                width: percent(1.0),
                height: length(130.0),
            },
            ..Container::default_style()
        }
    }
}
