use pix_engine::{ shape::Point, state::PixState, vector::Vector };

type Result<T> = pix_engine::prelude::PixResult<T>;
pub trait PixExtensions {
    ///draw text, without moving the cursor down a line. returns the size of the rendered text

    fn h_text<S>(&mut self, text: S) -> Result<(u32, u32)> where S: AsRef<str>;
    fn offset(&mut self, offset: Point);
}

impl PixExtensions for PixState {
    fn h_text<S>(&mut self, text: S) -> Result<(u32, u32)> where S: AsRef<str> {
        let mut cursor_pos = self.cursor_pos();
        let (width, height) = self.text(text)?;
        cursor_pos.offset_x(width as i32);
        self.set_cursor_pos(cursor_pos);
        return Ok((width, height));
    }

    fn offset(&mut self, offset: Point) {
        let cursor = self.cursor_pos();
        self.cursor_pos().offset(offset);
        self.set_cursor_pos(cursor);
    }
}
