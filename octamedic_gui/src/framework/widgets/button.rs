use crate::framework::widget::{Widget, WidgetCore};
use crate::framework::widgets::container::Container;
use crate::framework::widgets::Label::Label;
use ggez::graphics::{Canvas, Color};
use ggez::{Context, GameResult};
use taffy::prelude::{auto, length};
use taffy::{AlignItems, JustifyContent, Layout, Rect, Size, Style, TaffyResult, TaffyTree};

pub struct Button {
    core: WidgetCore,
    background_colors: ButtonColors,
    text_colors: ButtonColors,
    on_click: Box<dyn Fn()>,
    container: Container,
}
impl Button {
    pub fn new<F>(taffy: &mut TaffyTree, label: &str, font_size: f32, on_click: F) -> Self
    where
        F: Fn() + 'static,
    {
        let child = Label::new(taffy, label, font_size).boxed();
        let container = Container::new(taffy, vec![child]).with_style(taffy, Self::default_style());

        let core = *container.core();
        return Self {
            background_colors: ButtonColors::background(),
            text_colors: ButtonColors::text(),
            on_click: Box::new(on_click),
            core,
            container,
        };
    }
}
pub struct ButtonColors {
    default_color: Color,
    hover_color: Color,
    press_color: Color,
}
impl ButtonColors {
    pub fn background() -> Self {
        return Self {
            default_color: Color::WHITE,
            hover_color: Color::from_rgb(211, 211, 211),
            press_color: Color::BLACK,
        };
    }
    pub fn text() -> Self {
        return Self {
            default_color: Color::BLACK,
            hover_color: Color::BLACK,
            press_color: Color::WHITE,
        };
    }
}

impl Widget for Button {
    fn core(&self) -> &WidgetCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }
    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {
        let layout = self.container.layout(taffy)?;
        self.core = *self.container.core();
        return Ok(layout);
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        self.container.set_colors(
            self.text_colors.default_color,
            self.background_colors.default_color,
        );
        
        self.container.update(ctx, taffy)?;
        return Ok(());
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        return self.container.render(ctx, canvas);
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
            padding: Rect {
                left: length(2.0),
                right: length(2.0),
                bottom: length(1.0),
                top: length(1.0),
            },
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            ..Container::default_style()
        }
    }
}
