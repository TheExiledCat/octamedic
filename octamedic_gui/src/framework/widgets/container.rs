use ggez::{
    Context, GameResult,
    graphics::{Color, DrawMode, DrawParam, Mesh},
};
use taffy::{
    Display, JustifyContent, Layout, NodeId, Size, Style, TaffyResult, TaffyTree,
    prelude::{length, percent},
};

use crate::framework::{
    input::InputEvent,
    widget::{Border, DefaultStyle, Widget, WidgetCore, WidgetSignal, handle_mouse_event},
};

pub struct Container {
    core: WidgetCore,
    children: Vec<Box<dyn Widget>>,
    border_thickness: f32,
}

impl Container {
    pub fn new(taffy: &mut TaffyTree, children: Vec<Box<dyn Widget>>) -> Self {

        let child_nodes: Vec<NodeId> = children.iter().map(|c| c.core().node).collect();

        let node = taffy
            .new_with_children(Self::default_style(), &child_nodes)
            .unwrap();

        let mut core = WidgetCore::new(node);

        core.style.background_color = Color::from_rgba(0, 0, 0, 0);

        return Self {
            core,
            children,
            border_thickness: 0.0,
        };
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {

        self.children.push(child);
    }

    pub fn child(&self, index: usize) -> &dyn Widget {

        return self.children[index].as_ref();
    }

    pub fn child_mut(&mut self, index: usize) -> &mut dyn Widget {

        return self.children[index].as_mut();
    }

    pub fn child_by_key(&self, key: NodeId) -> &dyn Widget {

        self.children
            .iter()
            .find(|c| c.core().node == key)
            .unwrap()
            .as_ref()
    }

    pub fn child_by_key_mut(&mut self, key: NodeId) -> &mut dyn Widget {

        self.children
            .iter_mut()
            .find(|c| c.core().node == key)
            .unwrap()
            .as_mut()
    }

    pub fn remove_child(&mut self, taffy: &mut TaffyTree, index: usize) {

        let child = self.children.remove(index);

        taffy.remove(child.core().node).unwrap();
    }
}

impl Widget for Container {
    fn core(&self) -> &WidgetCore {

        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {

        &mut self.core
    }

    fn handle_event(&mut self, event: &InputEvent) -> bool {

        for child in self.children.iter_mut() {

            if child.handle_event(event) {

                return true;
            }
        }

        if handle_mouse_event(self.core_mut(), event) {

            return true;
        }

        return false;
    }

    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {

        for child in &mut self.children {

            child.layout(taffy)?;
        }

        let layout = taffy.layout(self.core().node)?;

        self.core_mut().rect = self.get_screen_rect(taffy);

        if self.core().just_pressed {

            self.core_mut().just_pressed = false;
        }

        return Ok(layout.clone());
    }

    fn update(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {

        for child in self.children.iter_mut() {

            child.update(ctx, taffy)?;
        }

        return Ok(());
    }

    fn render(
        &self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
    ) -> ggez::GameResult {

        let rect = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            self.core.rect,
            self.core.style.background_color,
        )?;

        canvas.draw(&rect, DrawParam::default());

        for child in &self.children {

            child.render(ctx, canvas)?;
        }

        if self.border_thickness > 0.0 {

            let border = Mesh::new_rectangle(
                ctx,
                DrawMode::stroke(self.border_thickness),
                self.core.rect,
                self.core.style.foreground_color,
            )?;

            canvas.draw(&border, DrawParam::default());
        }

        return Ok(());
    }

    fn set_colors(&mut self, fg_color: Color, bg_color: Color) {

        let style = self.core().style;

        let style = style.with_foreground(fg_color).with_background(bg_color);

        self.core_mut().style = style;

        for child in &mut self.children {

            child.set_colors(fg_color, child.core().style.background_color);
        }
    }

    fn set_text(&mut self, text: &str) -> () {

        for child in self.children.iter_mut() {

            child.set_text(text);
        }
    }

    fn on_mount(&mut self, ctx: &mut Context, taffy: &mut TaffyTree) -> GameResult {

        for child in self.children.iter_mut() {

            child.on_mount(ctx, taffy)?
        }

        return Ok(());
    }

    fn collect_signals(&mut self) -> Vec<WidgetSignal> {

        let mut signals = Vec::new();

        for child in self.children.iter_mut() {

            signals.extend(child.collect_signals());
        }

        return signals;
    }

    fn handle_signals(&mut self, signals: &Vec<WidgetSignal>) {

        for child in self.children.iter_mut() {

            child.handle_signals(&signals);
        }
    }
}

impl Border for Container {
    fn with_border(mut self, color: Color, thickness: f32) -> Self
    where
        Self: Sized,
    {

        self.border_thickness = thickness;

        self.core.style.foreground_color = color;

        return self;
    }

    fn set_border(&mut self, border: Color, thickness: f32) {

        self.core.style.foreground_color = border;

        self.border_thickness = thickness;
    }
}

impl DefaultStyle for Container {
    fn default_style() -> Style
    where
        Self: Sized,
    {

        Style {
            flex_direction: taffy::FlexDirection::Row,
            display: Display::Flex,
            justify_content: Some(JustifyContent::Start),
            gap: Size::zero(),
            size: Size {
                width: percent(1.0),
                height: length(20.0),
            },

            ..Default::default()
        }
    }
}
