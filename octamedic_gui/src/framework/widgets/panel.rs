use std::ops::BitOr;

use ggez::graphics::{Canvas, Color, DrawParam, Quad, Rect};
use taffy::{
    Display, JustifyContent, Size, Style,
    prelude::{length, percent},
};

use crate::framework::widget::{DrawState, Widget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]

pub struct Borders(u8);

impl Borders {
    pub const NONE: Self = Self(0);

    pub const TOP: Self = Self(1);

    pub const BOTTOM: Self = Self(2);

    pub const LEFT: Self = Self(4);

    pub const RIGHT: Self = Self(8);

    pub const ALL: Self = Self(15);

    pub fn contains(self, other: Self) -> bool {

        return self.0 & other.0 == other.0;
    }
}

impl BitOr for Borders {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {

        return Self(self.0 | other.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]

pub struct BorderSide {
    pub color: Color,
    pub thickness: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]

struct SideBorders {
    top: Option<BorderSide>,
    bottom: Option<BorderSide>,
    left: Option<BorderSide>,
    right: Option<BorderSide>,
}

fn thickness(side: Option<BorderSide>) -> f32 {

    return side.map_or(0.0, |side| side.thickness);
}

pub struct Panel {
    background: Color,
    borders: SideBorders,
}

impl Panel {
    pub fn new() -> Self {

        return Self {
            background: Color::from_rgba(0, 0, 0, 0),
            borders: SideBorders::default(),
        };
    }

    pub fn with_background(mut self, color: Color) -> Self {

        self.background = color;

        return self;
    }

    pub fn with_border(self, color: Color, thickness: f32) -> Self {

        return self.with_borders(Borders::ALL, color, thickness);
    }

    // only changes the sides in `borders`, so sides can be given different colors by chaining
    pub fn with_borders(mut self, borders: Borders, color: Color, thickness: f32) -> Self {

        let side = Some(BorderSide {
            color,
            thickness,
        });

        let sides = [
            (Borders::TOP, &mut self.borders.top),
            (Borders::BOTTOM, &mut self.borders.bottom),
            (Borders::LEFT, &mut self.borders.left),
            (Borders::RIGHT, &mut self.borders.right),
        ];

        for (border, slot) in sides {

            if borders.contains(border) {

                *slot = side;
            }
        }

        return self;
    }
}

impl Widget for Panel {
    fn style(&self) -> Style {

        let borders = self.borders;

        return Style {
            border: taffy::Rect {
                top: length(thickness(borders.top)),
                bottom: length(thickness(borders.bottom)),
                left: length(thickness(borders.left)),
                right: length(thickness(borders.right)),
            },
            flex_direction: taffy::FlexDirection::Row,
            display: Display::Flex,
            justify_content: Some(JustifyContent::Start),
            gap: Size::zero(),
            size: Size {
                width: percent(1.0_f32),
                height: length(20.0_f32),
            },
            ..Default::default()
        };
    }

    fn draw(&self, canvas: &mut Canvas, state: &DrawState) {

        if self.background.a > 0.0 {

            fill_rect(canvas, state.rect, self.background);
        }
    }

    fn draw_after_children(&self, canvas: &mut Canvas, state: &DrawState) {

        let Rect {
            x,
            y,
            w,
            h,
        } = state.rect;

        let borders = self.borders;

        if let Some(side) = borders.left {

            fill_rect(canvas, Rect::new(x, y, side.thickness, h), side.color);
        }

        if let Some(side) = borders.right {

            fill_rect(
                canvas,
                Rect::new(x + w - side.thickness, y, side.thickness, h),
                side.color,
            );
        }

        if let Some(side) = borders.top {

            fill_rect(canvas, Rect::new(x, y, w, side.thickness), side.color);
        }

        if let Some(side) = borders.bottom {

            fill_rect(
                canvas,
                Rect::new(x, y + h - side.thickness, w, side.thickness),
                side.color,
            );
        }
    }
}

pub fn fill_rect(canvas: &mut Canvas, rect: Rect, color: Color) {

    canvas.draw(&Quad, DrawParam::new().dest_rect(rect).color(color));
}

#[cfg(test)]

mod tests {

    use taffy::LengthPercentage;

    use super::*;

    fn border_widths(panel: &Panel) -> [LengthPercentage; 4] {

        let border = panel.style().border;

        return [border.top, border.bottom, border.left, border.right];
    }

    #[test]

    fn only_the_chosen_sides_get_a_border() {

        let panel = Panel::new().with_borders(Borders::LEFT | Borders::RIGHT, Color::RED, 2.0);

        assert_eq!(
            border_widths(&panel),
            [length(0.0), length(0.0), length(2.0), length(2.0)]
        );
    }

    #[test]

    fn chaining_sets_each_sides_own_color() {

        let panel =
            Panel::new()
                .with_border(Color::RED, 1.0)
                .with_borders(Borders::TOP, Color::BLUE, 3.0);

        assert_eq!(
            panel.borders.top,
            Some(BorderSide {
                color: Color::BLUE,
                thickness: 3.0,
            })
        );

        assert_eq!(
            panel.borders.left,
            Some(BorderSide {
                color: Color::RED,
                thickness: 1.0,
            })
        );
    }

    #[test]

    fn borders_flags_combine() {

        let sides = Borders::TOP | Borders::BOTTOM;

        assert!(sides.contains(Borders::TOP));

        assert!(!sides.contains(Borders::LEFT));

        assert!(Borders::ALL.contains(sides));

        assert!(Borders::ALL.contains(Borders::NONE));
    }
}
