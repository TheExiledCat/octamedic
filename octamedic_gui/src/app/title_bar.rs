use std::time::Instant;

use ggez::graphics::{Canvas, Color};
use taffy::{
    AlignItems, NodeId, Rect, Size, Style,
    prelude::{auto, length, percent},
};

use crate::framework::{
    input::MouseButton,
    ui::{Ui, UiEvent},
    widget::{DrawState, Widget},
    widgets::{button::Button, label::Label, panel::Panel},
};

pub struct TitleBar {
    background: Panel,
    timer: Option<NodeId>,
    started: Instant,
    displayed_seconds: Option<u64>,
}

impl TitleBar {
    pub fn new() -> Self {

        return Self {
            background: Panel::new().with_background(Color::BLACK),
            timer: None,
            started: Instant::now(),
            displayed_seconds: None,
        };
    }

    pub fn style() -> Style {

        return Style {
            size: Size {
                width: percent(1.0_f32),
                height: auto(),
            },
            align_items: Some(AlignItems::Center),
            gap: Size::length(1.0_f32),
            padding: Rect {
                left: length(3.0_f32),
                right: length(3.0_f32),
                top: length(2.0_f32),
                bottom: length(2.0_f32),
            },
            ..Panel::default_style()
        };
    }
}

impl Widget for TitleBar {
    fn mount(&mut self, ui: &mut Ui, bar: NodeId) {

        ui.add(bar, Label::default_style(), Label::new("TIMER", 24.0));

        self.timer = Some(ui.add(bar, Button::default_style(), Button::new("00:00", 24.0)));

        ui.add(
            bar,
            Label::default_style(),
            Label::new("+-OctaMEDIC Professional (v4.00)", 28.0),
        );

        ui.add(
            bar,
            Style {
                margin: Rect {
                    left: auto(),
                    ..Rect::zero()
                },
                ..Label::default_style()
            },
            Label::new("By Amano Rosuko", 18.0),
        );
    }

    fn draw(&self, canvas: &mut Canvas, state: &DrawState) {

        self.background.draw(canvas, state);
    }

    fn update(&mut self, ui: &mut Ui, _node: NodeId) {

        let seconds = self.started.elapsed().as_secs();

        if self.displayed_seconds == Some(seconds) {

            return;
        }

        self.displayed_seconds = Some(seconds);

        if let Some(timer) = self.timer {

            Button::set_text(
                ui,
                timer,
                format!("{:02}:{:02}", seconds / 60, seconds % 60),
            );
        }
    }

    fn on_event(&mut self, _ui: &mut Ui, _node: NodeId, event: &UiEvent) -> bool {

        let Some(timer) = self.timer else {

            return false;
        };

        let reset = UiEvent::Clicked {
            node: timer,
            button: MouseButton::Left,
        };

        if *event != reset {

            return false;
        }

        self.started = Instant::now();

        self.displayed_seconds = None;

        return true;
    }
}
