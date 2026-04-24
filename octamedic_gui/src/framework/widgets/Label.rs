use crate::app::app::TOPAZ_FONT_KEY;
use crate::framework::widget::{Widget, WidgetCore};
use ggez::graphics::{Canvas, DrawParam, Text};
use ggez::{Context, GameResult};
use taffy::prelude::{auto, length};
use taffy::{Size, Style, TaffyTree};

pub struct Label {
    core: WidgetCore,
    text: String,
    font_size: f32,
}
impl Label {
    pub fn new(taffy: &mut TaffyTree, text: &str, font_size: f32) -> Self {
        let node = taffy.new_leaf(Self::default_style()).unwrap();

        let core = WidgetCore::new(node);
        return Self {
            text: text.to_owned(),
            font_size,
            core,
        };
    }
    fn text(&self) -> Text {
        let mut text = Text::new(&self.text);
        text.set_wrap(false)
            .set_font(TOPAZ_FONT_KEY)
            .set_scale(self.font_size);

        return text;
    }
}

impl Widget for Label {
    fn core(&self) -> &WidgetCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        let text = self.text();
        let size = text.measure(ctx)?;
        let rect = &mut self.core.rect;
        rect.w = size.x;
        rect.h = size.y;
        let mut style = taffy.style(self.core.node).unwrap().clone();
        style.size.width = length(size.x);
        style.size.height = length(size.y);
        taffy.set_style(self.core.node, style).unwrap();
        return Ok(());
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let text = self.text();
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([self.core.rect.x, self.core.rect.y])
                .color(self.core.style.foreground_color),
        );

        return Ok(());
    }

    fn default_style() -> Style
    where
        Self: Sized,
    {
        Style {
            size: Size {
                width: auto(),
                height: auto(),
            },

            ..Default::default()
        }
    }
}
