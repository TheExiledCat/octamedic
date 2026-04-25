use ggez::graphics::Color;
use ggez::{
    graphics::{Canvas, Rect}, Context,
    GameResult,
};
use taffy::{Layout, NodeId, Style, TaffyResult, TaffyTree};

use crate::framework::input::{InputEvent, MouseButton};
use crate::framework::widgets::container::Container;

#[derive(Clone, Copy)]
pub struct WidgetCore {
    pub node: NodeId,
    pub rect: Rect,
    pub disabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub just_pressed: bool,
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
            just_pressed: Default::default(),
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
        if self.core().just_pressed {
            self.core_mut().just_pressed = false;
        }
        return Ok(layout.clone());
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult;
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult;

    fn boxed(self) -> Box<Self>
    where
        Self: Sized,
    {
        return Box::new(self);
    }

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
    fn set_text(&mut self, text: &str) -> () {}
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
    fn on_mount(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        Ok(())
    }
    fn collect_signals(&mut self) -> Vec<WidgetSignal> {
        return vec![];
    }
    fn handle_signals(&mut self, signals: &Vec<WidgetSignal>) {}
}
pub struct WidgetSignal {
    pub node: NodeId,
    pub kind: WidgetSignalKind,
}
pub enum WidgetSignalKind {
    Clicked(MouseButton),
}

pub fn handle_mouse_event(core: &mut WidgetCore, event: &InputEvent) -> bool {
    match event {
        InputEvent::MouseMove { pos, delta } => {
            core.hovered = core.rect.contains(*pos);
        }
        InputEvent::MouseDown { pos, button } => {
            if !core.disabled
                && core.rect.contains(*pos)
                && let MouseButton::Left = button
            {
                core.pressed = true;
                core.just_pressed = true;
                return true;
            }
        }
        InputEvent::MouseUp { pos, button } => {
            core.pressed = false;
            if !core.disabled && core.pressed {
                return true;
            }
        }
        _ => (),
    }
    return false;
}
pub trait Component {
    fn container(&self) -> &Container;
    fn container_mut(&mut self) -> &mut Container;
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        return Ok(());
    }
    fn on_mount(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        return Ok(());
    }
    fn handle_signals(&mut self, signals: &Vec<WidgetSignal>) {}
    fn node_clicked(
        &mut self,
        signals: &Vec<WidgetSignal>,
        node: NodeId,
        button: MouseButton,
    ) -> bool {
        if let Some(s) = signals.iter().find(|s| s.node == node) {
            if let WidgetSignalKind::Clicked(b) = &s.kind {
                if button == *b {
                    return true;
                }
            }
        }
        return false;
    }
}
impl<C: Component> Widget for C {
    fn core(&self) -> &WidgetCore {
        self.container().core()
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        self.container_mut().core_mut()
    }
    fn handle_event(&mut self, event: &InputEvent) -> bool {
        self.container_mut().handle_event(event)
    }
    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {
        self.container_mut().layout(taffy)
    }
    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        self.container_mut().update(ctx, taffy)?;
        Component::update(self, ctx, taffy)
    }
    fn render(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        self.container().render(ctx, canvas)
    }
    fn set_text(&mut self, text: &str) {
        self.container_mut().set_text(text)
    }

    fn get_screen_rect(&self, tree: &TaffyTree) -> Rect {
        self.container().get_screen_rect(tree)
    }
    fn on_mount(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {
        self.container_mut().on_mount(ctx, taffy)
    }
    fn collect_signals(&mut self) -> Vec<WidgetSignal> {
        return self.container_mut().collect_signals();
    }
    fn handle_signals(&mut self, signals: &Vec<WidgetSignal>) {
        self.container_mut().handle_signals(signals);
        Component::handle_signals(self, signals)
    }
}

pub trait DefaultStyle {
    fn default_style() -> Style
    where
        Self: Sized;
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

pub trait Border {
    fn with_border(self, color: Color, thickness: f32) -> Self
    where
        Self: Sized;
    fn set_border(&mut self, border: Color, thickness: f32);
}

pub trait IntoColorHex {
    fn as_hex(&self) -> [u8; 4];
}
pub trait FromHex {
    fn hex(hex: impl IntoColorHex) -> Self
    where
        Self: Sized;
}

impl<T: ToString> IntoColorHex for T {
    fn as_hex(&self) -> [u8; 4] {
        let str = self.to_string();
        let count = str.chars().count();
        match count {
            3 | 4 => {
                let byte1 = u8::from_str_radix(&str[0..1], 16).unwrap() * 17;
                let byte2 = u8::from_str_radix(&str[1..2], 16).unwrap() * 17;
                let byte3 = u8::from_str_radix(&str[2..3], 16).unwrap() * 17;
                let byte4 = if count == 4 {
                    u8::from_str_radix(&str[3..4], 16).unwrap() * 17
                } else {
                    u8::MAX
                };
                [byte1, byte2, byte3, byte4]
            }

            6 | 8 => {
                let byte1 = u8::from_str_radix(&str[0..2], 16).unwrap();
                let byte2 = u8::from_str_radix(&str[2..4], 16).unwrap();
                let byte3 = u8::from_str_radix(&str[4..6], 16).unwrap();
                let byte4 = if count == 8 {
                    u8::from_str_radix(&str[6..8], 16).unwrap()
                } else {
                    u8::MAX
                };
                [byte1, byte2, byte3, byte4]
            }
            _ => panic!("invalid hex string"),
        }
    }
}

impl FromHex for Color {
    fn hex(hex: impl IntoColorHex) -> Self {
        let hex = hex.as_hex();
        return Self::from_rgba(hex[0], hex[1], hex[2], hex[3]);
    }
}
