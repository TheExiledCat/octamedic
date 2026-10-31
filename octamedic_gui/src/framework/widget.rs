use crate::{
    app::state::AppState,
    framework::{event::Event, painter::Painter, rect::Rect},
};

#[allow(unused_variables)]
pub trait OctamedicWidget {
    fn update(&mut self, state: &mut AppState) {}

    /// True if the event was taken, so no other widget gets it.
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return false;
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter);
}
