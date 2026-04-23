use pix_engine::{ color::Color, engine::PixEngine, prelude::{ Font, RectMode }, rect, vector };

use crate::framework::{
    widget::{ ContainerWidget, WidgetKind },
    widgets::{ container::Container, title_bar::TitleBar },
};
const TOPAZ_CHAR_WIDTH: u32 = 4;

const TOPAZ_FONT: &[u8] = include_bytes!("../../assets/fonts/amiga-topaz.ttf");

pub struct App {
    size: vector::Vector,
    scale: f32,
    root: Box<dyn ContainerWidget>,
}

impl App {
    pub fn new() -> Self {
        let root = Self::root();
        Self { size: vector!(640.0, 201.0), scale: 1.0, root }
    }
    fn root() -> Box<dyn ContainerWidget> {
        let container = Container::new(rect!(0, 0, 100, 0), Color::PINK);
        return Box::new(container);
    }
    pub fn scale(&self) -> f32 {
        return self.scale;
    }
}
impl PixEngine for App {
    fn on_start(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        s.font_family(Font::from_bytes("amiga-topaz", TOPAZ_FONT))?;
        s.cursor(None)?;
        let (w, h) = s.display_dimensions()?;
        let scale_x = (w as f32) / (self.size.x() as f32);
        let scale_y = (h as f32) / (self.size.y() as f32);
        self.scale = scale_x.max(scale_y);

        self.root.resize(s, s.width()?, s.height()?);
        self.root.add_child(WidgetKind::Leaf(Box::new(TitleBar::new())));
        self.root.init(s, self, rect!(0, 0, 0, 0));
        return Ok(());
    }
    fn on_update(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        s.clip(None)?;
        s.rect_mode(RectMode::Corner);
        let (w, h) = s.dimensions()?;
        self.root.render(s, self, rect!(0, 0, w as i32, h as i32))?;

        return Ok(());
    }
}
