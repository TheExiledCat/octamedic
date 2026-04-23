use pix_engine::{ shape::Rect, state::PixState };

use crate::app::app::App;

pub trait Widget {
    fn init(
        &mut self,
        s: &mut PixState,
        app: &mut App,
        rect: Rect
    ) -> pix_engine::prelude::PixResult<()> {
        return Ok(());
    }
    fn render(&self, s: &mut PixState, app: &App, rect: Rect) -> pix_engine::prelude::PixResult<()>;
    fn get_rect(&self, s: &mut PixState, app: &mut App) -> pix_engine::prelude::PixResult<Rect>;
}
pub trait ContainerWidget: Widget {
    fn get_children(&self) -> &Vec<WidgetKind>;
    fn add_child(&mut self, child: WidgetKind);
    fn resize(&mut self, s: &mut PixState, width: u32, height: u32);
}

pub enum WidgetKind {
    Container(Box<dyn ContainerWidget>),
    Leaf(Box<dyn Widget>),
}

impl Widget for WidgetKind {
    fn render(
        &self,
        s: &mut PixState,
        app: &App,
        rect: Rect
    ) -> pix_engine::prelude::PixResult<()> {
        match self {
            WidgetKind::Container(container_widget) => container_widget.render(s, app, rect),
            WidgetKind::Leaf(widget) => widget.render(s, app, rect),
        }
    }

    fn get_rect(&self, s: &mut PixState, app: &mut App) -> pix_engine::prelude::PixResult<Rect> {
        match self {
            WidgetKind::Container(container_widget) => container_widget.get_rect(s, app),
            WidgetKind::Leaf(widget) => widget.get_rect(s, app),
        }
    }
}
