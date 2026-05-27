use ggez::graphics::Color;
use taffy::{
    FlexDirection, Rect, Size, Style, TaffyTree,
    prelude::{auto, length, percent},
};

use crate::framework::{
    widget::{Border, Component, DefaultStyle, FromHex, Widget},
    widgets::{Label::Label, container::Container},
};

pub struct Editor {
    container: Container,
}

impl Editor {
    pub fn new(taffy: &mut TaffyTree) -> Self {

        let border = Color::BLACK;

        let fill = Color::hex("000"); //Color::from_rgb(211, 211, 211);
        let cols = {

            let mut cols: Vec<Box<dyn Widget>> = vec![];

            let mut is_row_number = true;

            for col in 0..9 {

                let texts = {

                    let mut labels: Vec<Box<dyn Widget>> = vec![];

                    for i in 0..15 {

                        let t = if is_row_number {

                            format!("{:03} ", i)
                        } else {

                            "--- 00000".into()
                        };

                        labels.push(Label::new(taffy, &t, 19.0).boxed());
                    }

                    labels
                };

                if is_row_number {

                    is_row_number = false;
                }

                let col = Container::new(taffy, texts)
                    .with_style(
                        taffy,
                        Style {
                            size: Size {
                                width: auto(),
                                height: percent(1.0),
                            },
                            margin: taffy::Rect {
                                left: length(1.0),
                                ..Rect::zero()
                            },
                            flex_direction: FlexDirection::Column,
                            ..Container::default_style()
                        },
                    )
                    .with_border(Color::hex("E4A0B7"), 1.0)
                    .boxed();

                cols.push(col);
            }

            cols
        };

        let row = Container::new(taffy, cols)
            .with_style(
                taffy,
                Style {
                    size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    gap: Size::zero(),
                    ..Container::default_style()
                },
            )
            .with_colors(border, Color::BLACK)
            .boxed();

        let container = Container::new(taffy, vec![row])
            .with_style(taffy, Self::default_style())
            .with_colors(border, fill);

        return Self {
            container,
        };
    }
}

impl Component for Editor {
    fn container(&self) -> &Container {

        &self.container
    }

    fn container_mut(&mut self) -> &mut Container {

        &mut self.container
    }
}

impl DefaultStyle for Editor {
    fn default_style() -> Style
    where
        Self: Sized,
    {

        Style {
            size: Size {
                width: percent(1.0),
                height: auto(),
            },
            margin: Rect {
                left: length(2.0),
                ..Rect::zero()
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Container::default_style()
        }
    }
}
