use pix_engine::{ engine::Engine, prelude::* };

use app::App;

mod app;
fn main() -> PixResult<()> {
    let mut engine = Engine::builder().fullscreen().title("OctaMEDIC").show_frame_rate().build()?;

    let mut app = App::new();
    return engine.run(&mut app);
}
