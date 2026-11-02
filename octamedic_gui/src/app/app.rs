use std::time::{Duration, Instant};

use ggez::{
    Context, GameResult,
    event::{EventHandler, MouseButton},
    graphics::{Canvas, Color, DrawParam, Image, Sampler},
    input::{keyboard::KeyInput, mouse},
};
use octamedic_core::{
    amiga::clock::VideoStandard, data::project::OctamedicProject, engine::handle::EngineHandle,
};
use rustuition::{
    gadget::Gadget,
    painter::Painter,
    rect::Point,
    screen::{ScaleMode, Viewport},
    target::BatchedCanvas,
};

use crate::{
    app::{DEFAULT_VIDEO, main_view::MainView, screen, state::AppState},
    framework::{
        dialog_stack::DialogStack,
        event::{self, Event},
        key_repeat::KeyRepeat,
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
    scale_mode: ScaleMode,
    viewport: Viewport,
    // in screen pixels
    pointer: Point,
    next_frame: Instant,
    // text input events don't say whether the key is repeating
    key_repeated: bool,
    key_repeat: KeyRepeat,
    // the text event after a held back repeat gets dropped too
    repeat_held_back: bool,
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
            scale_mode: ScaleMode::default(),
            viewport: Viewport::default(),
            pointer: [0, 0],
            next_frame: Instant::now(),
            key_repeated: false,
            key_repeat: KeyRepeat::default(),
            repeat_held_back: false,
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
        let (width, height) = ctx.gfx.drawable_size();
        self.viewport = screen(self.video).viewport([width, height], self.scale_mode);
    }

    fn follow_video(&mut self, ctx: &Context) {
        if self.state.video() == self.video {
            return;
        }
        self.video = self.state.video();
        self.screen = screen_image(ctx, self.video);
        self.fit_viewport(ctx);
    }

    fn dispatch(&mut self, event: Event) -> bool {
        return self.dialogs.handle_event(&mut self.state, &event)
            || self.main.handle_event(&mut self.state, &event);
    }

    fn mouse_event(&mut self, x: f32, y: f32, event: impl FnOnce(Point) -> Event) {
        self.pointer = screen(self.video).to_screen(self.viewport, [x, y]);
        self.dispatch(event(self.pointer));
    }

    fn render_screen(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_image(ctx, self.screen.clone(), Color::BLACK);
        canvas.set_sampler(Sampler::nearest_clamp());
        {
            let area = screen(self.video).rect();
            let mut target = BatchedCanvas::new(&ctx.gfx, &mut canvas, area);
            let mut painter = Painter::new(&mut target, &self.state.palette);
            self.main.render(&self.state, area, &mut painter);
            self.dialogs.render(&self.state, area, &mut painter);
        }
        // after the batched fills are flushed, so it's on top
        let [x, y] = self.pointer;
        let cursor = DrawParam::new()
            .dest([x as f32, y as f32])
            .scale(CURSOR_SCALE);
        canvas.draw(&self.cursor, cursor);
        return canvas.finish(ctx);
    }
}

impl EventHandler for App {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        ctx.gfx
            .set_window_title(&format!("OctaMEDIC ({:.02} FPS) ", ctx.time.fps()));
        self.state.update();
        self.main.update(&mut self.state);
        self.dialogs.update(&mut self.state);
        self.follow_video(ctx);
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
        return Ok(());
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.mouse_event(x, y, |pos| Event::MouseMove { pos });
        return Ok(());
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        // presses in the bars around the screen would land on its edge
        if !self.viewport.contains([x, y]) {
            return Ok(());
        }
        if let Some(button) = mouse_button(button) {
            self.mouse_event(x, y, |pos| Event::MouseDown { pos, button });
        }
        return Ok(());
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if let Some(button) = mouse_button(button) {
            self.mouse_event(x, y, |pos| Event::MouseUp { pos, button });
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
            mods: input.mods,
            repeat: repeated,
        };
        self.dispatch(event);
        return Ok(());
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(key) = input.keycode {
            self.key_repeat.released(key);
        }
        return Ok(());
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        if self.repeat_held_back {
            return Ok(());
        }
        self.dispatch(Event::Text {
            character,
            mods: ctx.keyboard.active_mods(),
            repeat: self.key_repeated,
        });
        return Ok(());
    }
}

fn mouse_button(button: MouseButton) -> Option<event::MouseButton> {
    return match button {
        MouseButton::Left => Some(event::MouseButton::Left),
        MouseButton::Right => Some(event::MouseButton::Right),
        _ => None,
    };
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
