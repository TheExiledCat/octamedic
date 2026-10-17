use ggez::graphics::{Canvas, Color};
use taffy::{
    AlignItems, JustifyContent, NodeId, Rect, Size, Style,
    prelude::{auto, length},
};

use crate::{
    app::state::AppState,
    framework::{
        input::MouseButton,
        ui::{Ui, UiEvent},
        widget::{DrawState, Widget},
        widgets::{label::Label, panel::fill_rect},
    },
};

type ClickHandler = Box<dyn FnMut(&mut Ui, &mut AppState, NodeId, MouseButton) -> bool>;

pub struct Button {
    initial_text: String,
    font_size: f32,
    label: Option<NodeId>,
    on_click: Option<ClickHandler>,
    background_colors: ButtonColors,
    text_colors: ButtonColors,
}

impl Button {
    pub fn new(text: impl Into<String>, font_size: f32) -> Self {

        return Self {
            initial_text: text.into(),
            font_size,
            label: None,
            on_click: None,
            background_colors: ButtonColors::background(),
            text_colors: ButtonColors::text(),
        };
    }

    // the handler returns whether it handled the click; if not, or without a handler, the click
    // bubbles on to the button's ancestors
    pub fn on_click(
        mut self,
        handler: impl FnMut(&mut Ui, &mut AppState, NodeId, MouseButton) -> bool + 'static,
    ) -> Self {

        self.on_click = Some(Box::new(handler));

        return self;
    }

    pub fn label_node(&self) -> Option<NodeId> {

        return self.label;
    }

    pub fn set_text(ui: &mut Ui, node: NodeId, text: impl Into<String>) {

        let Some(label) = ui.get::<Button>(node).and_then(Button::label_node) else {

            return;
        };

        if let Some(label) = ui.get_mut::<Label>(label) {

            label.set_text(text);
        }
    }
}

impl Widget for Button {
    fn style(&self) -> Style {

        return Style {
            size: Size {
                width: auto(),
                height: auto(),
            },
            padding: Rect {
                left: length(2.0_f32),
                right: length(2.0_f32),
                bottom: length(1.0_f32),
                top: length(1.0_f32),
            },
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };
    }

    fn mount(&mut self, ui: &mut Ui, node: NodeId) {

        let label = Label::new(std::mem::take(&mut self.initial_text), self.font_size)
            .with_color(self.text_colors.default_color);

        self.label = Some(ui.add(node, label));
    }

    fn update(&mut self, ui: &mut Ui, _state: &mut AppState, node: NodeId) {

        let color = self
            .text_colors
            .for_state(ui.is_hovered(node), ui.is_pressed(node));

        let Some(label) = self.label else {

            return;
        };

        // only through `get_mut` when it changes, since that marks the label for layout.
        if ui
            .get::<Label>(label)
            .is_some_and(|label| label.color() != color)
            && let Some(label) = ui.get_mut::<Label>(label)
        {

            label.set_color(color);
        }
    }

    fn draw(&self, canvas: &mut Canvas, draw: &DrawState, _state: &AppState) {

        let background = self.background_colors.for_state(draw.hovered, draw.pressed);

        fill_rect(canvas, draw.rect, background);
    }

    fn on_event(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        node: NodeId,
        event: &UiEvent,
    ) -> bool {

        let UiEvent::Clicked {
            node: clicked,
            button,
        } = *event;

        if clicked != node {

            return false;
        }

        let Some(handler) = &mut self.on_click else {

            return false;
        };

        return handler(ui, state, node, button);
    }

    fn is_interactive(&self) -> bool {

        return true;
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

    fn for_state(&self, hovered: bool, pressed: bool) -> Color {

        return if pressed {

            self.press_color
        } else if hovered {

            self.hover_color
        } else {

            self.default_color
        };
    }
}
