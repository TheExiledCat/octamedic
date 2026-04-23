use pix_engine::{
    color::Color,
    engine::PixEngine,
    image::PixelFormat,
    point,
    prelude::{ Font, RectMode },
    rect,
    shape::Rect,
    state::PixState,
    vector,
};

const TOPAZ_BYTES: &[u8] = include_bytes!("../assets/fonts/amiga-topaz.ttf");

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
        s.font_family(Font::from_bytes("amiga-topaz", TOPAZ_BYTES))?;
        return Ok(());
    }
    fn on_update(
        &mut self,
        s: &mut pix_engine::prelude::PixState
    ) -> pix_engine::prelude::PixResult<()> {
        s.background(Color::BLACK);
        s.clip(None)?;
        s.rect_mode(RectMode::Corner);

        let (w, h) = s.display_dimensions()?;

        let scale_x = (w as f32) / (self.size.x() as f32);
        let scale_y = (w as f32) / (self.size.y() as f32);

        s.fill(Color::PINK);
        let w = self.size.x();
        let h = self.size.y();
        let rect = rect!(vector!(0, 0), w as i32, h as i32);
        s.rect(rect)?;

        let pos = s.mouse_pos();
        s.fill(Color::RED);
        s.font_size(2 * (scale_x.max(scale_y) as u32))?;

        s.text(format!("w: {}, h: {}, mx: {}, my: {}", w, h, pos.x(), pos.y()))?;
        s.line([
            [0, 0],
            [self.size.x() as i32, self.size.y() as i32],
        ])?;
        s.fill(Color::WHITE);
        self.place(pos.x(), pos.y());
        return s.rect(self.block);
    }
}

trait TextPoint {
    fn text_point<S>(&mut self, text: S) -> pix_engine::prelude::PixResult<()> where S: AsRef<str>;
}
