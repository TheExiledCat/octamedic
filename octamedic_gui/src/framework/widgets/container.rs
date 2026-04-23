use pix_engine::{
    color::Color,
    image::PixelFormat,
    rect,
    shape::{ Point, Rect },
    texture::TextureId,
};

use crate::framework::widget::{ ContainerWidget, Widget, WidgetKind };

pub struct Container {
    children: Vec<WidgetKind>,
    color: Color,
    texture: Option<TextureId>,
    rect: Rect,
}
impl Container {
    pub fn new(rect: Rect, color: Color) -> Self {
        Self { children: vec![], color, texture: None, rect }
    }
    fn create_texture(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        if self.texture.is_some() {
            s.delete_texture(self.texture.unwrap())?;
        }
        self.texture = Some(
            s.create_texture(
                self.rect.width() as u32,
                self.rect.height() as u32,
                PixelFormat::Rgba
            )?
        );
        return Ok(());
    }
}
impl Widget for Container {
    fn init(
        &mut self,
        s: &mut pix_engine::prelude::PixState,
        app: &mut crate::app::app::App,
        rect: pix_engine::prelude::Rect
    ) -> pix_engine::prelude::PixResult<()> {
        self.create_texture(s)?;
        return Ok(());
    }
    fn render(
        &self,
        s: &mut pix_engine::prelude::PixState,
        app: &crate::app::app::App,
        rect: pix_engine::prelude::Rect
    ) -> pix_engine::prelude::PixResult<()> {
        let tex = self.texture.unwrap();
        s.set_texture_target(tex)?;
        s.background(self.color);
        s.clear()?;
        for child in &self.children {
            child.render(s, app, self.rect)?;
        }
        s.clear_texture_target();
        s.texture(tex, None, self.rect)?;
        return Ok(());
    }

    fn get_rect(
        &self,
        s: &mut pix_engine::prelude::PixState,
        app: &mut crate::app::app::App
    ) -> pix_engine::prelude::PixResult<pix_engine::prelude::Rect> {
        return Ok(self.rect);
    }
}
impl ContainerWidget for Container {
    fn get_children(&self) -> &Vec<WidgetKind> {
        return &self.children;
    }

    fn add_child(&mut self, child: WidgetKind) {
        self.children.push(child);
    }

    fn resize(&mut self, s: &mut pix_engine::prelude::PixState, width: u32, height: u32) {
        self.rect = self.rect.resize(width as i32, height as i32);
        self.create_texture(s).unwrap();
    }
}
