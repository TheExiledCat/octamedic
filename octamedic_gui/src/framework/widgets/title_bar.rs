use crate::framework::input::MouseButton;
use crate::framework::widget::{Component, DefaultStyle, Widget, WidgetSignal};
use crate::framework::widgets::button::Button;
use crate::framework::widgets::container::Container;
use crate::framework::widgets::Label::Label;
use ggez::graphics::Color;
use ggez::{Context, GameResult};
use std::time::Instant;
use taffy::prelude::{auto, length, percent};
use taffy::{AlignItems, NodeId, Rect, Size, Style, TaffyTree};

pub struct TitleBar {
    container: Container,
    button_node: NodeId,
    time_start: Instant,
}

impl TitleBar {
    pub fn new(taffy: &mut TaffyTree) -> Self {
        let timer_text = Label::new(taffy, "TIMER", 24.0).boxed();
        let timer_button = Button::new(taffy, "00:00", 24.0, || {}).boxed();
        let button_node = timer_button.core().node;
        let title = Label::new(taffy, "+-OctaMEDIC Professional (v4.00)", 28.0)
            .with_style(
                taffy,
                Style {
                    margin: Rect { ..Rect::zero() },
                    ..Label::default_style()
                },
            )
            .boxed();
        let author = Label::new(taffy, "By Amano Rosuko", 18.0)
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
            container,
            time_start: Instant::now(),
            button_node,
        };
    }
}

impl Component for TitleBar {
    fn container(&self) -> &Container {
        &self.container
    }

    fn container_mut(&mut self) -> &mut Container {
        &mut self.container
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        let total_seconds = self.time_start.elapsed().as_secs();
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        self.container
            .child_mut(1)
            .set_text(&format!("{:02}:{:02}", minutes, seconds));
        return Ok(());
    }
    fn handle_signals(&mut self, signals: &Vec<WidgetSignal>) {
        self.container_mut().handle_signals(signals);
        if self.node_clicked(signals, self.button_node, MouseButton::Left) {
            self.time_start = Instant::now();
        }
    }
    fn on_mount(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        self.time_start = Instant::now();
        return Ok(());
    }
}
impl DefaultStyle for TitleBar {
    fn default_style() -> Style
    where
        Self: Sized,
    {
        return Style {
            size: Size {
                width: percent(1.0),
                height: auto(),
            },

            align_items: Some(AlignItems::Center),
            gap: Size::length(1.0),
            padding: Rect {
                left: length(3.0),
                right: length(3.0),
                top: length(2.0),
                bottom: length(2.0),
            },
            ..Container::default_style()
        };
    }
}
