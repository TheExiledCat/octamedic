use crate::framework::widget::{Widget, WidgetCore};
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};
use ggez::{Context, GameResult};
use rand::RngExt;
use taffy::prelude::{length, percent};
use taffy::{Display, Layout, NodeId, Size, Style, TaffyResult, TaffyTree};

pub struct Container {
    core: WidgetCore,
    children: Vec<Box<dyn Widget>>,
}
impl Container {
    pub fn new(taffy: &mut TaffyTree, children: Vec<Box<dyn Widget>>) -> Self {
        let child_nodes: Vec<NodeId> = children.iter().map(|c| c.core().node).collect();
        let node = taffy
            .new_with_children(Self::default_style(), &child_nodes)
            .unwrap();
        let core = WidgetCore::new(node);
        return Self { core, children };
    }
    pub fn child(&self, index: usize) -> &dyn Widget {
        return self.children[index].as_ref();
    }
    pub fn child_mut(&mut self, index: usize) -> &dyn Widget {
        return self.children[index].as_mut();
    }
}
impl Widget for Container {
    fn core(&self) -> &WidgetCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }
    fn layout(&mut self, taffy: &TaffyTree) -> TaffyResult<Layout> {
        for child in &mut self.children {
            child.layout(taffy)?;
        }
        let layout = taffy.layout(self.core().node)?;
        self.core_mut().rect = self.get_screen_rect(taffy);
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
    fn default_style() -> Style
    where
        Self: Sized,
    {
        Style {
            flex_direction: taffy::FlexDirection::Row,
            display: Display::Flex,

            gap: Size {
                height: length(8.0),
                width: length(0.0),
            },
            size: Size {
                width: percent(1.0),
                height: length(20.0),
            },
            ..Default::default()
        }
    }
}
