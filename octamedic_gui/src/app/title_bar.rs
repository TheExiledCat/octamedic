use std::time::Instant;

use ggez::graphics::{Canvas, Color};
use taffy::{
    AlignItems, NodeId, Rect, Size, Style,
    prelude::{auto, length, percent},
};

use crate::{
    app::state::AppState,
    framework::{
        ui::Ui,
        widget::{DrawState, Widget},
        widgets::{button::Button, label::Label, panel::Panel},
    },
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

    pub fn restart_timer(&mut self) {

        self.started = Instant::now();

        self.displayed_seconds = None;
    }
}

impl Widget for TitleBar {
    fn style(&self) -> Style {

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
            ..self.background.style()
        };
    }

    fn mount(&mut self, ui: &mut Ui, bar: NodeId) {

        ui.add(bar, Label::new("TIMER", 24.0));

        let timer = Button::new("00:00", 24.0).on_click(|ui, _state, node, _| {

            if let Some(bar) = ui.ancestor_mut::<TitleBar>(node) {

                bar.restart_timer();
            }

            true
        });

        self.timer = Some(ui.add(bar, timer));

        ui.add(bar, Label::new("+-OctaMEDIC Professional (v4.00)", 28.0));

        ui.add_styled(bar, Label::new("By Amano Rosuko", 18.0), |style| {

            style.margin.left = auto();
        });
    }

    fn draw(&self, canvas: &mut Canvas, draw: &DrawState, state: &AppState) {

        self.background.draw(canvas, draw, state);
    }

    fn update(&mut self, ui: &mut Ui, _state: &mut AppState, _node: NodeId) {

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
}
