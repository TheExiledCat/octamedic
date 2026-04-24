use ggez::graphics::{ Color, DrawMode, DrawParam, Mesh };
use rand::RngExt;
use taffy::{ NodeId, Size, Style, TaffyTree, prelude::length };

use crate::framework::widget::{ Widget, WidgetCore };

pub struct Container {
    core: WidgetCore,
    children: Vec<Box<dyn Widget>>,
}
impl Container {
    pub fn new(taffy: &mut TaffyTree, children: Vec<Box<dyn Widget>>) -> Self {
        let child_nodes: Vec<NodeId> = children
            .iter()
            .map(|c| c.core().node)
            .collect();
        let node = taffy
            .new_with_children(
                Style {
                    flex_direction: taffy::FlexDirection::Row,
                    gap: Size {
                        height: length(8.0),
                        width: length(0.0),
                    },
                    ..Default::default()
                },
                &child_nodes
            )
            .unwrap();
        return Self { core: WidgetCore::new(node), children };
    }
}
impl Widget for Container {
    fn core(&self) -> &WidgetCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }

    fn render(
        &self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas
    ) -> ggez::GameResult {
        let mut ran = rand::rng();
        let r = ran.random_range(0..255);
        let g = ran.random_range(0..255);
        let b = ran.random_range(0..255);
        let color = Color::from_rgb(r, g, b);
        let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), self.core.rect, color)?;
        canvas.draw(&rect, DrawParam::default());
        return Ok(());
    }
}
