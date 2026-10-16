use ggez::graphics::{Canvas, Color, DrawParam, Quad, Rect};
use taffy::{
    Display, JustifyContent, Size, Style,
    prelude::{length, percent},
};

use crate::framework::widget::{DrawState, Widget};

pub struct Panel {
    background: Color,
    border: Option<(Color, f32)>,
}

impl Panel {
    pub fn new() -> Self {

        return Self {
            background: Color::from_rgba(0, 0, 0, 0),
            border: None,
        };
    }

    pub fn with_background(mut self, color: Color) -> Self {

        self.background = color;

        return self;
    }

    pub fn with_border(mut self, color: Color, thickness: f32) -> Self {

        self.border = Some((color, thickness));

        return self;
    }

    pub fn default_style() -> Style {

        return Style {
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
}

impl Widget for Panel {
    fn draw(&self, canvas: &mut Canvas, state: &DrawState) {

        if self.background.a > 0.0 {

            fill_rect(canvas, state.rect, self.background);
        }
    }

    fn draw_after_children(&self, canvas: &mut Canvas, state: &DrawState) {

        let Some((color, thickness)) = self.border else {

            return;
        };

        let Rect {
            x,
            y,
            w,
            h,
        } = state.rect;

        fill_rect(canvas, Rect::new(x, y, w, thickness), color);

        fill_rect(canvas, Rect::new(x, y + h - thickness, w, thickness), color);

        fill_rect(canvas, Rect::new(x, y, thickness, h), color);

        fill_rect(canvas, Rect::new(x + w - thickness, y, thickness, h), color);
    }
}

pub fn fill_rect(canvas: &mut Canvas, rect: Rect, color: Color) {

    canvas.draw(&Quad, DrawParam::new().dest_rect(rect).color(color));
}
