use ggez::{
    Context, GameResult,
    graphics::{Canvas, Color},
};
use taffy::{
    AlignItems, JustifyContent, Layout, Rect, Size, Style, TaffyResult, TaffyTree,
    prelude::{auto, length},
};

use crate::framework::{
    input::{InputEvent, MouseButton::Left},
    widget::{DefaultStyle, Widget, WidgetCore, WidgetSignal, WidgetSignalKind},
    widgets::{Label::Label, container::Container},
};

pub struct Button {
    background_colors: ButtonColors,
    text_colors: ButtonColors,
    on_click: Box<dyn Fn()>,
    container: Container,
}

impl Button {
    pub fn new<F>(taffy: &mut TaffyTree, label: impl ToString, font_size: f32, on_click: F) -> Self
    where
        F: Fn() + 'static,
    {

        let child = Self::get_label(taffy, label, font_size).boxed();

        let container = Container::new(taffy, vec![child]).with_style(taffy, Self::default_style());

        return Self {
            background_colors: ButtonColors::background(),
            text_colors: ButtonColors::text(),
            on_click: Box::new(on_click),
            container,
        };
    }

    fn get_label(taffy: &mut TaffyTree, text: impl ToString, font_size: f32) -> Label {

        Label::new(taffy, text, font_size)
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

        &self.container.core()
    }

    fn core_mut(&mut self) -> &mut WidgetCore {

        self.container.core_mut()
    }

    fn handle_event(&mut self, event: &InputEvent) -> bool {

        self.container.handle_event(event)
    }

    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {

        self.container.layout(taffy)
    }

    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {

        if self.core().pressed {

            self.container.set_colors(
                self.text_colors.press_color,
                self.background_colors.press_color,
            )
        } else if self.core().hovered {

            self.container.set_colors(
                self.text_colors.hover_color,
                self.background_colors.hover_color,
            )
        } else {

            self.container.set_colors(
                self.text_colors.default_color,
                self.background_colors.default_color,
            );
        }

        self.container.update(ctx, taffy)?;

        return Ok(());
    }

    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {

        return self.container.render(ctx, canvas);
    }

    fn set_text(&mut self, text: &str) {

        self.container.set_text(text)
    }

    fn collect_signals(&mut self) -> Vec<WidgetSignal> {

        if self.core().just_pressed {

            return vec![WidgetSignal {
                node: self.core().node,
                kind: WidgetSignalKind::Clicked(Left),
            }];
        }

        vec![]
    }
}

impl DefaultStyle for Button {
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
