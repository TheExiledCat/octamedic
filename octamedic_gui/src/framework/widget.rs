use ggez::{ Context, GameResult, graphics::{ Canvas, Rect } };
use taffy::{ Layout, NodeId, TaffyResult, TaffyTree };

use crate::framework::input::InputEvent;

pub struct WidgetCore {
    pub node: NodeId,
    pub rect: Rect,
    pub disabled: bool,
    pub hovered: bool,
    pub pressed: bool,
}
impl WidgetCore {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            rect: Default::default(),
            disabled: Default::default(),
            hovered: Default::default(),
            pressed: Default::default(),
        }
    }
}
pub trait Widget {
    fn core(&self) -> &WidgetCore;
    fn core_mut(&mut self) -> &mut WidgetCore;
    fn handle_event(&mut self, event: &InputEvent) -> bool {
        if handle_mouse_event(self.core_mut(), event) {
            return true;
        }
        return false;
    }

    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<()> {
        let layout = taffy.layout(self.core().node)?;
        self.core_mut().rect = layout.into_rect();
        return Ok(());
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult;

    fn boxed(self) -> Box<Self> where Self: Sized {
        return Box::new(self);
    }
}

fn handle_mouse_event(core: &mut WidgetCore, event: &InputEvent) -> bool {
    match event {
        InputEvent::MouseMove { x, y, dx, dy } => {
            core.hovered = core.rect.contains([*x as f32, *y as f32]);
        }
        InputEvent::MouseDown { x, y, button } => {
            if !core.disabled && core.rect.contains([*x as f32, *y as f32]) {
                core.pressed = true;
                return true;
            }
        }
        InputEvent::MouseUp { x, y, button } => {
            if !core.disabled && core.pressed && core.rect.contains([*x as f32, *y as f32]) {
                core.pressed = false;
                return true;
            }
            core.pressed = false;
        }
        _ => (),
    }
    return false;
}

trait IntoRect {
    fn into_rect(&self) -> Rect;
}
impl IntoRect for Layout {
    fn into_rect(&self) -> Rect {
        return Rect {
            x: self.location.x,
            y: self.location.y,
            w: self.size.width,
            h: self.size.height,
        };
    }
}
