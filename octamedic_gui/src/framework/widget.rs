use crate::{
    app::state::AppState,
    framework::{event::Event, painter::Painter, rect::Rect},
};

#[allow(unused_variables)]
pub trait OctamedicWidget {
    fn update(&mut self, state: &mut AppState) {}

    /// Returns whether the event was taken, which stops it going to any other widget.
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return false;
    }

    /// Draws the widget from `state` into `area`, every frame.
    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter);
}
