use pix_engine::{ color::Color, point, rect, shape::Rect };

use crate::{ framework::widget::Widget, util::pix_extensions::PixExtensions };

pub struct TitleBar {}
impl TitleBar {
    pub fn new() -> Self {
        return Self {};
    }
}
impl Widget for TitleBar {
    fn render(
        &self,
        s: &mut pix_engine::prelude::PixState,
        app: &crate::app::app::App,
        rect: pix_engine::prelude::Rect
    ) -> pix_engine::prelude::PixResult<()> {
        s.wrap(None);
        s.fill(Color::WHITE);
        s.font_size((5.0 * app.scale()) as u32)?;
        s.h_text("TIMER")?;
        s.offset(point!(10, 0));
        s.h_text("OctaMEDIC Professional (4.00)")?;
        return Ok(());
    }

    fn get_rect(
        &self,
        s: &mut pix_engine::prelude::PixState,
        app: &mut crate::app::app::App
    ) -> pix_engine::prelude::PixResult<Rect> {
        return Ok(rect!(0, 0, s.width()? as i32, 50));
    }
}
