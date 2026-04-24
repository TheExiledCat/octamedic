use crate::framework::widget::{Widget, WidgetCore};
use crate::framework::widgets::button::Button;
use crate::framework::widgets::container::Container;
use crate::framework::widgets::Label::Label;
use ggez::graphics::{Canvas, Color};
use ggez::{Context, GameResult};
use taffy::prelude::{auto, length, percent};
use taffy::{AlignItems, Layout, Rect, Size, Style, TaffyResult, TaffyTree};

pub struct TitleBar {
    core: WidgetCore,
    container: Container,
    //timer: TimerWidget
}

impl TitleBar {
    pub fn new(taffy: &mut TaffyTree) -> Self {
        let timer_text = Label::new(taffy, "TIMER", 16.0).boxed();
        let timer_button = Button::new(taffy, "00:00", 16.0, || {}).boxed();
        let title = Label::new(taffy, "OctaMEDIC Professional (v4.00)", 16.0)
            .with_style(
                taffy,
                Style {
                    margin: Rect {
                        left: length(10.0),
                        ..Rect::zero()
                    },
                    ..Label::default_style()
                },
            )
            .boxed();
        let author = Label::new(taffy, "By Amano Rosuko", 16.0)
            .with_style(
                taffy,
                Style {
                    margin: Rect {
                        left: auto(),
                        ..Rect::zero()
                    },

                    ..Label::default_style()
                },
            )
            .boxed();
        let children: Vec<Box<dyn Widget>> = vec![timer_text, timer_button, title, author];

        let container = Container::new(taffy, children)
            .with_style(taffy, Self::default_style())
            .with_colors(Color::WHITE, Color::BLACK);
        return Self {
            core: WidgetCore::new(container.core().node),
            container,
        };
    }
}

impl Widget for TitleBar {
    fn core(&self) -> &WidgetCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        self.container.update(ctx, taffy)
    }
    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {
        let layout = self.container.layout(taffy)?;
        self.core = *self.container.core();
        return Ok(layout);
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let w = self.container.core();
        return self.container.render(ctx, canvas);
        
    }

    fn default_style() -> Style
    where
        Self: Sized,
    {
        return Style {
            size: Size {
                width: percent(1.0),
                height: auto(),
            },
            gap: Size {
                width: length(20.0),
                height: length(0.0),
            },
            align_items: Some(AlignItems::Center),
            padding: Rect {
                left: length(20.0),
                right: length(20.0),
                top: length(0.0),
                bottom: length(0.0),
            },
            ..Container::default_style()
        };
    }
}
