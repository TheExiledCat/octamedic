use std::time::{Duration, Instant};

use ggez::{
    Context, GameError, GameResult,
    event::{EventHandler, MouseButton},
    graphics::{Canvas, Color, DrawParam, Image, ImageFormat, Sampler},
    input::{
        keyboard::{KeyCode, KeyInput, KeyMods},
        mouse,
    },
    winit::dpi::PhysicalSize,
};
use octamedic_core::{
    amiga::clock::VideoStandard, data::project::OctamedicProject, engine::handle::EngineHandle,
};
use octamedic_util::primitives::Flip;
use rustuition::{
    gadget::Gadget,
    painter::Painter,
    rect::{Point, Rect},
    screen::{ScaleMode, Viewport},
    target::{BatchedCanvas, Framebuffer, RenderTarget, Rgba},
};

use crate::{
    app::{
        DEFAULT_VIDEO, main_view::MainView, prompt, screen, state::AppState,
        window_shape::WindowShape,
    },
    framework::{
        dialog_stack::DialogStack,
        event::{self, Event, Wheel},
        key_repeat::KeyRepeat,
        keymap::amiga_mods,
    },
};

const CURSOR: &[u8] = include_bytes!("../../assets/cursor.png");

// the pointer sprite's pixels are twice as wide as the screen's
const CURSOR_SCALE: [f32; 2] = [2.0, 1.0];

pub struct App {
    state: AppState,
    main: MainView,
    dialogs: DialogStack,
    // native res, scaled up into the window
    screen: Image,
    video: VideoStandard,
    cursor: Image,
    // painted into the screen in memory when it's filtered
    cursor_pixels: CursorPixels,
    scale_mode: ScaleMode,
    viewport: Viewport,
    window_shape: WindowShape,
    // in screen pixels
    pointer: Point,
    next_frame: Instant,
    // text input events don't say whether the key is repeating
    key_repeated: bool,
    // the last key pressed went to a prompt
    prompt_took_key: bool,
    key_repeat: KeyRepeat,
    // the text event after a held back repeat gets dropped too
    repeat_held_back: bool,
    // the modifiers don't say which Alt, and right Alt is the Amiga key
    alts_held: [bool; 2],
}

impl App {
    pub fn new(ctx: &mut Context, project: OctamedicProject) -> GameResult<Self> {
        mouse::set_cursor_hidden(ctx, true);
        let mut app = Self {
            state: Self::state_with_engine(project),
            main: MainView::new(),
            dialogs: DialogStack::default(),
            screen: screen_image(ctx, DEFAULT_VIDEO),
            video: DEFAULT_VIDEO,
            cursor: Image::from_bytes(ctx, CURSOR)?,
            cursor_pixels: CursorPixels::decode(CURSOR)?,
            // fills the window at any size, not just whole steps
            scale_mode: ScaleMode::Fit,
            viewport: Viewport::default(),
            window_shape: WindowShape::new(drawable_size(ctx)),
            pointer: Point::new(0, 0),
            next_frame: Instant::now(),
            key_repeated: false,
            prompt_took_key: false,
            key_repeat: KeyRepeat::default(),
            repeat_held_back: false,
            alts_held: [false; 2],
        };
        app.fit_viewport(ctx);
        return Ok(app);
    }

    // no audio device, run silent instead of bailing
    fn state_with_engine(project: OctamedicProject) -> AppState {
        return match EngineHandle::spawn(DEFAULT_VIDEO.clock()) {
            Ok(engine) => AppState::new(project, engine),
            Err(error) => {
                let mut state = AppState::new(project, EngineHandle::disconnected());
                state.notify(format!("NO AUDIO: {error}"));
                state
            }
        };
    }

    // vsync is off, frames get paced to the PAL or NTSC rate by hand. fixed schedule so
    // oversleeping doesn't pile up, a late frame just restarts it, no catching up
    fn wait_for_next_frame(&mut self) {
        self.next_frame += Duration::from_secs(1) / self.video.clock().frame_rate;
        let now = Instant::now();
        match self.next_frame.checked_duration_since(now) {
            Some(rest) => std::thread::sleep(rest),
            None => self.next_frame = now,
        }
    }

    fn fit_viewport(&mut self, ctx: &Context) {
        self.viewport = screen(self.video).viewport(drawable_size(ctx), self.scale_mode);
    }

    // a windowed window goes back to the screen's shape once it's been resized
    fn keep_window_shape(&mut self, ctx: &Context) {
        let window = ctx.gfx.window();
        let free = !window.is_maximized() && window.fullscreen().is_none();
        let limit = window.current_monitor().map(|monitor| {
            let (outer, inner, area) = (window.outer_size(), window.inner_size(), monitor.size());
            // less the title bar and borders
            let room = |area: u32, outer: u32, inner: u32| {
                return area.saturating_sub(outer.saturating_sub(inner)) as f32;
            };
            [
                room(area.width, outer.width, inner.width),
                room(area.height, outer.height, inner.height),
            ]
        });
        let shape = screen(self.video).window_size(1.0);
        let settled = self.window_shape.settle(Instant::now(), shape, limit, free);
        if let Some([width, height]) = settled {
            window.set_inner_size(PhysicalSize::new(width as u32, height as u32));
        }
    }

    fn follow_video(&mut self, ctx: &Context) {
        if self.state.video() == self.video {
            return;
        }
        self.video = self.state.video();
        self.screen = screen_image(ctx, self.video);
        self.fit_viewport(ctx);
        // NTSC's screen is a different shape
        self.window_shape
            .resized(Instant::now(), drawable_size(ctx));
    }

    fn track_alt(&mut self, key: KeyCode, held: bool) {
        match key {
            KeyCode::LAlt => self.alts_held[0] = held,
            KeyCode::RAlt => self.alts_held[1] = held,
            _ => {}
        }
    }

    fn with_amiga(&self, mods: KeyMods) -> KeyMods {
        let [left, right] = self.alts_held;
        return amiga_mods(mods, left, right);
    }

    fn dispatch(&mut self, event: Event) -> bool {
        return prompt::handle_event(&mut self.state, &event)
            || self.dialogs.handle_event(&mut self.state, &event)
            || self.main.handle_event(&mut self.state, &event);
    }

    fn mouse_event(&mut self, ctx: &Context, x: f32, y: f32, event: impl FnOnce(Point) -> Event) {
        self.state.mouse_mods = self.with_amiga(ctx.keyboard.active_mods());
        self.pointer = screen(self.video).to_screen(self.viewport, [x, y]);
        self.dispatch(event(self.pointer));
    }

    // painted in memory instead of straight onto the GPU, so the filters get the finished pixels
    fn filtered_screen(&mut self, ctx: &Context, area: Rect) -> Image {
        let mut frame = Framebuffer::new(area.w, area.h);
        {
            let mut painter = Painter::new(&mut frame, &self.state.palette);
            self.main.render(&self.state, area, &mut painter);
            self.dialogs.render(&self.state, area, &mut painter);
        }
        self.cursor_pixels.paint(&mut frame, self.pointer);
        self.state.screen_filters.apply(&mut frame);
        let rgba: Vec<u8> = frame
            .pixels()
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, u8::MAX])
            .collect();
        return Image::from_pixels(
            ctx,
            &rgba,
            ImageFormat::Rgba8UnormSrgb,
            area.w as u32,
            area.h as u32,
        );
    }

    fn render_screen(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_image(ctx, self.screen.clone(), Color::BLACK);
        canvas.set_sampler(Sampler::nearest_clamp());
        let area = screen(self.video).rect();
        if self.state.screen_filters.is_active() {
            let filtered = self.filtered_screen(ctx, area);
            canvas.draw(&filtered, DrawParam::new());
            return canvas.finish(ctx);
        }
        {
            let mut target = BatchedCanvas::new(&ctx.gfx, &mut canvas, area);
            let mut painter = Painter::new(&mut target, &self.state.palette);
            self.main.render(&self.state, area, &mut painter);
            self.dialogs.render(&self.state, area, &mut painter);
        }
        // after the batched fills are flushed, so it's on top
        let Point { x, y } = self.pointer;
        let cursor = DrawParam::new()
            .dest([x as f32, y as f32])
            .scale(CURSOR_SCALE);
        canvas.draw(&self.cursor, cursor);
        return canvas.finish(ctx);
    }
}

impl EventHandler for App {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        ctx.gfx.set_window_title(&format!(
            "OctaMEDIC ({} {:.0} Hz) ",
            self.video,
            ctx.time.fps()
        ));
        self.state.update();
        self.state.screen_filters.advance_frame();
        self.main.update(&mut self.state);
        self.dialogs.update(&mut self.state);
        self.follow_video(ctx);
        self.keep_window_shape(ctx);
        if self.state.exit_requested {
            ctx.request_quit();
        }
        return Ok(());
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.render_screen(ctx)?;
        let mut frame = Canvas::from_frame(ctx, Color::BLACK);
        frame.set_sampler(Sampler::nearest_clamp());
        let Viewport { x, y, w, h } = self.viewport;
        let screen = screen(self.video);
        frame.draw(
            &self.screen,
            DrawParam::new()
                .dest([x, y])
                .scale([w / screen.width as f32, h / screen.height as f32]),
        );
        frame.finish(ctx)?;
        self.wait_for_next_frame();
        return Ok(());
    }

    fn resize_event(&mut self, ctx: &mut Context, _width: f32, _height: f32) -> GameResult {
        self.fit_viewport(ctx);
        self.window_shape
            .resized(Instant::now(), drawable_size(ctx));
        return Ok(());
    }

    fn mouse_motion_event(
        &mut self,
        ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.mouse_event(ctx, x, y, |pos| Event::MouseMove { pos });
        return Ok(());
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        // presses in the bars around the screen would land on its edge
        if !self.viewport.contains([x, y]) {
            return Ok(());
        }
        if let Some(button) = mouse_button(button) {
            self.mouse_event(ctx, x, y, |pos| Event::MouseDown { pos, button });
        }
        return Ok(());
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if let Some(button) = mouse_button(button) {
            self.mouse_event(ctx, x, y, |pos| Event::MouseUp { pos, button });
        }
        return Ok(());
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, x: f32, y: f32) -> GameResult {
        let pointer = ctx.mouse.position();
        // like presses, the bars around the screen would land on its edge
        if self.viewport.contains([pointer.x, pointer.y]) {
            let delta = [x, y];
            self.mouse_event(ctx, pointer.x, pointer.y, |pos| {
                Event::Wheel(Wheel { pos, delta })
            });
        }
        return Ok(());
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: KeyInput,
        repeated: bool,
    ) -> GameResult {
        self.key_repeated = repeated;
        let Some(key) = input.keycode else {
            return Ok(());
        };
        self.track_alt(key, true);
        if key == KeyCode::Capital && !repeated {
            self.state.caps_lock.flip();
        }
        let now = Instant::now();
        let delay = self.state.video().clock().key_repeat_delay;
        self.repeat_held_back = repeated && !self.key_repeat.allows_repeat(key, now, delay);
        if self.repeat_held_back {
            return Ok(());
        }
        if !repeated {
            self.key_repeat.pressed(key, now);
        }
        let event = Event::Key {
            key,
            mods: self.with_amiga(input.mods),
            repeat: repeated,
        };
        self.prompt_took_key = self.state.prompt.is_some();
        self.dispatch(event);
        return Ok(());
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(key) = input.keycode {
            self.key_repeat.released(key);
            self.track_alt(key, false);
        }
        return Ok(());
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        // the letter of a key that answered a prompt isn't typed as well
        if self.repeat_held_back || self.prompt_took_key {
            return Ok(());
        }
        let shift = ctx.keyboard.active_mods().contains(KeyMods::SHIFT);
        if let Some(caps_lock) = caps_lock_from_letter(character, shift) {
            self.state.caps_lock = caps_lock;
        }
        self.dispatch(Event::Text {
            character,
            mods: self.with_amiga(ctx.keyboard.active_mods()),
            repeat: self.key_repeated,
        });
        return Ok(());
    }
}

// the key's toggle isn't in the modifiers, and it may already be on at startup, so a typed letter
// tells: uppercase without shift, or lowercase with it, means it's on
fn caps_lock_from_letter(character: char, shift: bool) -> Option<bool> {
    if !character.is_alphabetic() || character.is_uppercase() == character.is_lowercase() {
        return None;
    }
    return Some(character.is_uppercase() != shift);
}

fn mouse_button(button: MouseButton) -> Option<event::MouseButton> {
    return match button {
        MouseButton::Left => Some(event::MouseButton::Left),
        MouseButton::Right => Some(event::MouseButton::Right),
        _ => None,
    };
}

// the pointer sprite as RGBA pixels, so it can go through the filters with the rest of the screen
struct CursorPixels {
    width: i32,
    rgba: Vec<[u8; 4]>,
}

impl CursorPixels {
    fn decode(png: &[u8]) -> GameResult<Self> {
        let decoded = image::load_from_memory(png)
            .map_err(|error| GameError::ResourceLoadError(error.to_string()))?
            .to_rgba8();
        return Ok(Self {
            width: decoded.width() as i32,
            rgba: decoded.pixels().map(|pixel| pixel.0).collect(),
        });
    }

    fn paint(&self, frame: &mut Framebuffer, Point { x, y }: Point) {
        let [scale_x, scale_y] = CURSOR_SCALE.map(|scale| scale as i32);
        for (index, &[r, g, b, a]) in self.rgba.iter().enumerate() {
            if a == 0 {
                continue;
            }
            let (column, row) = (index as i32 % self.width, index as i32 / self.width);
            let pixel = Rect::new(x + column * scale_x, y + row * scale_y, scale_x, scale_y);
            frame.fill(pixel, Rgba { r, g, b, a });
        }
    }
}

fn screen_image(ctx: &Context, video: VideoStandard) -> Image {
    let screen = screen(video);
    return Image::new_canvas_image(
        ctx,
        ctx.gfx.surface_format(),
        screen.width as u32,
        screen.height as u32,
        1,
    );
}

fn drawable_size(ctx: &Context) -> [f32; 2] {
    let (width, height) = ctx.gfx.drawable_size();
    return [width, height];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_paints_into_the_framebuffer() {
        let cursor = CursorPixels::decode(CURSOR).unwrap();
        let mut frame = Framebuffer::new(64, 32);
        cursor.paint(&mut frame, Point::new(10, 5));
        let changed: Vec<(i32, i32)> = (0..32)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.pixel(x, y) != Some([0, 0, 0]))
            .collect();
        assert!(!changed.is_empty());
        let width = cursor.width * CURSOR_SCALE[0] as i32;
        assert!(
            changed
                .iter()
                .all(|&(x, y)| (10..10 + width).contains(&x) && y >= 5)
        );
        // twice as wide as the sprite, like the GPU draws it
        assert!(changed.iter().any(|&(x, _)| x >= 10 + cursor.width));
    }

    #[test]
    fn letters_tell_whether_caps_lock_is_on() {
        assert_eq!(caps_lock_from_letter('A', false), Some(true));
        assert_eq!(caps_lock_from_letter('a', false), Some(false));
        assert_eq!(caps_lock_from_letter('a', true), Some(true));
        assert_eq!(caps_lock_from_letter('A', true), Some(false));
        assert_eq!(caps_lock_from_letter('5', false), None);
    }
}
