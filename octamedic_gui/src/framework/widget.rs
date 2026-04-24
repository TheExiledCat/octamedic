use ggez::graphics::Color;
use ggez::{
    graphics::{Canvas, Rect}, Context,
    GameResult,
};
use taffy::{Layout, NodeId, Style, TaffyResult, TaffyTree};

use crate::framework::input::InputEvent;

#[derive(Clone, Copy)]
pub struct WidgetCore {
    pub node: NodeId,
    pub rect: Rect,
    pub disabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub style: WidgetStyle,
}
impl WidgetCore {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            rect: Default::default(),
            disabled: Default::default(),
            hovered: Default::default(),
            pressed: Default::default(),
            style: WidgetStyle::new(),
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
    
    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {
        let layout = taffy.layout(self.core().node)?;
        self.core_mut().rect = self.get_screen_rect(taffy);
        return Ok(layout.clone());
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        return Ok(());
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult;

    fn boxed(self) -> Box<Self>
    where
        Self: Sized,
    {
        return Box::new(self);
    }
    fn default_style() -> Style
    where
        Self: Sized;
    fn with_style(self, taffy: &mut TaffyTree, style: Style) -> Self
    where
        Self: Sized,
    {
        let node = self.core().node;
        taffy.set_style(node, style).unwrap();

        return self;
    }
    fn with_colors(mut self, fg_color: Color, bg_color: Color) -> Self
    where
        Self: Sized,
    {
        let style = self
            .core()
            .style
            .with_foreground(fg_color)
            .with_background(bg_color);
        self.core_mut().style = style;
        return self;
    }
    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {
        let style = self.core().style;
        let style = style.with_foreground(fg_color).with_background(bg_color);
        self.core_mut().style = style;
    }

    fn get_screen_rect(&self, tree: &TaffyTree) -> Rect {
        let mut x = 0.0;
        let mut y = 0.0;
        let node = self.core().node;
        let mut current = Some(node);

        while let Some(n) = current {
            let layout = tree.layout(n).unwrap();

            x += layout.location.x;
            y += layout.location.y;

            current = tree.parent(n);
        }
        let size = tree.layout(self.core().node).unwrap().size;
        return Rect::new(x, y, size.width, size.height);
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

pub trait IntoRect {
    fn as_rect(&self) -> Rect;
}
impl IntoRect for Layout {
    fn as_rect(&self) -> Rect {
        return Rect {
            x: self.location.x,
            y: self.location.y,
            w: self.size.width,
            h: self.size.height,
        };
    }
}

#[derive(Clone, Copy)]
pub struct WidgetStyle {
    pub updated: bool,
    pub background_color: Color,
    pub foreground_color: Color,
}

impl WidgetStyle {
    pub fn new() -> Self {
        Self {
            updated: false,
            background_color: Color::BLACK,
            foreground_color: Color::WHITE,
        }
    }
    pub fn with_foreground(mut self, color: Color) -> Self {
        self.updated = true;
        self.foreground_color = color;
        return self;
    }
    pub fn with_background(mut self, color: Color) -> Self {
        self.updated = true;
        self.background_color = color;
        return self;
    }
}
