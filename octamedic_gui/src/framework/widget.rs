use std::any::Any;

use ggez::{
    Context,
    graphics::{Canvas, Rect},
};
use taffy::{AvailableSpace, NodeId, Size, Style};

use crate::framework::ui::{Ui, UiEvent};

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {

        return self;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {

        return self;
    }
}

#[derive(Clone, Copy, Debug)]

pub struct DrawState {
    pub rect: Rect,
    pub content_rect: Rect,
    pub hovered: bool,
    pub pressed: bool,
}

pub trait Widget: AsAny {
    fn style(&self) -> Style {

        return Style::default();
    }

    fn mount(&mut self, _ui: &mut Ui, _node: NodeId) {}

    fn draw(&self, canvas: &mut Canvas, state: &DrawState);

    fn draw_after_children(&self, _canvas: &mut Canvas, _state: &DrawState) {}

    fn measure_content(
        &self,
        _ctx: &Context,
        known: Size<Option<f32>>,
        _available: Size<AvailableSpace>,
    ) -> Size<f32> {

        return Size {
            width: known.width.unwrap_or(0.0),
            height: known.height.unwrap_or(0.0),
        };
    }

    fn update(&mut self, _ui: &mut Ui, _node: NodeId) {}

    fn on_event(&mut self, _ui: &mut Ui, _node: NodeId, _event: &UiEvent) -> bool {

        return false;
    }

    fn is_interactive(&self) -> bool {

        return false;
    }
}
