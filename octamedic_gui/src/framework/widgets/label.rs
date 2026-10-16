use ggez::{
    Context,
    graphics::{Canvas, Color, DrawParam, Text},
};
use taffy::{AvailableSpace, Size};

use crate::framework::widget::{DrawState, Widget};

pub const TEXT_FONT: &str = "Topaz";

pub struct Label {
    text: String,
    font_size: f32,
    color: Color,
}

impl Label {
    pub fn new(text: impl Into<String>, font_size: f32) -> Self {

        return Self {
            text: text.into(),
            font_size,
            color: Color::WHITE,
        };
    }

    pub fn with_color(mut self, color: Color) -> Self {

        self.color = color;

        return self;
    }

    pub fn color(&self) -> Color {

        return self.color;
    }

    pub fn set_color(&mut self, color: Color) {

        self.color = color;
    }

    pub fn text(&self) -> &str {

        return &self.text;
    }

    pub fn set_text(&mut self, text: impl Into<String>) {

        self.text = text.into();
    }
}

impl Widget for Label {
    fn draw(&self, canvas: &mut Canvas, state: &DrawState) {

        draw_text(canvas, &self.text, self.font_size, self.color, state);
    }

    fn measure_content(
        &self,
        ctx: &Context,
        known: Size<Option<f32>>,
        _available: Size<AvailableSpace>,
    ) -> Size<f32> {

        return measure_text(ctx, &self.text, self.font_size, known);
    }
}

fn text(text: &str, font_size: f32) -> Text {

    let mut text = Text::new(text);

    text.set_wrap(false)
        .set_font(TEXT_FONT)
        .set_scale(font_size);

    return text;
}

pub fn draw_text(
    canvas: &mut Canvas,
    value: &str,
    font_size: f32,
    color: Color,
    state: &DrawState,
) {

    canvas.draw(
        &text(value, font_size),
        DrawParam::default()
            .dest([state.content_rect.x, state.content_rect.y])
            .color(color),
    );
}

pub fn measure_text(
    ctx: &Context,
    value: &str,
    font_size: f32,
    known: Size<Option<f32>>,
) -> Size<f32> {

    let measured = text(value, font_size)
        .measure(ctx)
        .unwrap_or([0.0, 0.0].into());

    return Size {
        width: known.width.unwrap_or(measured.x),
        height: known.height.unwrap_or(measured.y),
    };
}
