use pix_engine::{
    color::Color,
    engine::PixEngine,
    point,
    prelude::{ Font, RectMode },
    rect,
    shape::Rect,
    vector,
};

pub struct App {
    block: Rect,
    size: vector::Vector,
}

impl App {
    pub fn new() -> Self {
        Self { block: rect!(0, 0, 10, 10), size: vector!(640.0, 201.0) }
    }
    fn shift(&mut self, x_delta: i32, y_delta: i32) {
        self.block = self.block.offset(vector!(x_delta, y_delta));
    }
    fn place(&mut self, x: i32, y: i32) {
        self.block = self.block.reposition(x, y);
    }
}
impl PixEngine for App {
    fn on_start(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        return Ok(());
    }
    fn on_update(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        s.background(Color::BLACK);
        s.font_family(Font::NOTO)?;
        s.font_size(16)?;
        s.clip(None)?;
        s.rect_mode(RectMode::Corner);
        let (w, h) = s.display_dimensions()?;

        let scale_x = (w as f32) / (self.size.x() as f32);
        let scale_y = (w as f32) / (self.size.y() as f32);
        s.set_window_dimensions((w, h))?;
        s.set_cursor_pos(point!(0, 0));
        s.scale(scale_x, scale_y)?;
        s.fill(Color::PINK);
        let w = self.size.x();
        let h = self.size.y();
        let rect = rect!(vector!(0, 0), w as i32, h as i32);
        s.rect(rect)?;

        let pos = s.mouse_pos();
        s.fill(Color::RED);
        s.text(format!("w: {}, h: {}, mx: {}, my: {}", w, h, pos.x(), pos.y()))?;
        s.fill(Color::WHITE);
        self.place(pos.x(), pos.y());
        return s.rect(self.block);
    }
}
