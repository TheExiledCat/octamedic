use crate::{event::MouseEvent, painter::Painter, rect::Rect};

/// Anything on screen. `S` is the app's state, which gadgets show and change, and `E` its events,
/// which rustuition's own gadgets only need the mouse out of (see
/// [`PointerEvent`](crate::event::PointerEvent)).
#[allow(unused_variables)]
pub trait Gadget<S, E = MouseEvent> {
    /// Runs every frame before `render`, for copying things out of the state.
    fn update(&mut self, state: &mut S) {}

    /// Handles an event, true if it's taken and nobody else should see it.
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        return false;
    }

    /// Draws it in `area`, remembering where for clicks.
    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter);
}
