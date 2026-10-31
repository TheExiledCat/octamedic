use std::time::{Duration, Instant};

use ggez::{
    Context, GameResult,
    event::{EventHandler, MouseButton},
    graphics::{Canvas, Color, DrawParam, Image, Rect as WindowRect, Sampler},
    input::{keyboard::KeyInput, mouse},
};
use octamedic_core::{
    amiga::clock::AmigaClock, data::project::OctamedicProject, engine::handle::EngineHandle,
};

use crate::{
    app::{main_view::MainView, state::AppState},
    framework::{
        event::{self, Event},
        painter::Painter,
        rect::Point,
        screen::{SCREEN, SCREEN_HEIGHT, SCREEN_WIDTH, ScaleMode, to_screen, viewport},
        target::BatchedCanvas,
        widget::OctamedicWidget,
    },
};

const CURSOR: &[u8] = include_bytes!("../../assets/cursor.png");

// the pointer sprite's pixels are twice as wide as the screen's
const CURSOR_SCALE: [f32; 2] = [2.0, 1.0];

// vsync is off, so frames are paced here, at the Amiga's display rate
const MACHINE: AmigaClock = AmigaClock::PAL;

pub struct App {
    state: AppState,
    main: MainView,
    // drawn at the screen's own size, then scaled into the window
    screen: Image,
    cursor: Image,
    scale_mode: ScaleMode,
    viewport: WindowRect,
    // in screen pixels
    pointer: Point,
    // when the next frame is due
    next_frame: Instant,
    // text input events don't say whether the key is repeating
    key_repeated: bool,
}

impl App {
    pub fn new(ctx: &mut Context, project: OctamedicProject) -> GameResult<Self> {
        mouse::set_cursor_hidden(ctx, true);
        let screen = Image::new_canvas_image(
            ctx,
            ctx.gfx.surface_format(),
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
            1,
        );
        let mut app = Self {
            state: Self::state_with_engine(project),
            main: MainView::new(),
            screen,
            cursor: Image::from_bytes(ctx, CURSOR)?,
            scale_mode: ScaleMode::default(),
            viewport: SCREEN.into(),
            pointer: [0, 0],
            next_frame: Instant::now(),
            key_repeated: false,
        };
        app.fit_viewport(ctx);
        return Ok(app);
    }

    // runs without sound rather than not at all when there's no audio device
    fn state_with_engine(project: OctamedicProject) -> AppState {
        return match EngineHandle::spawn() {
            Ok(engine) => AppState::new(project, engine),
            Err(error) => {
                let mut state = AppState::new(project, EngineHandle::disconnected());
                state.notify(format!("NO AUDIO: {error}"));
                state
            }
        };
    }

    // against a fixed schedule, so oversleeping doesn't add up; a frame that's late starts the
    // schedule again rather than rushing the next ones
    fn wait_for_next_frame(&mut self) {
        self.next_frame += Duration::from_secs(1) / MACHINE.frame_rate;
        let now = Instant::now();
        match self.next_frame.checked_duration_since(now) {
            Some(rest) => std::thread::sleep(rest),
            None => self.next_frame = now,
        }
    }

    fn fit_viewport(&mut self, ctx: &Context) {
        let (width, height) = ctx.gfx.drawable_size();
        self.viewport = viewport([width, height], self.scale_mode);
    }

    fn dispatch(&mut self, event: Event) -> bool {
        return self.main.handle_event(&mut self.state, &event);
    }

    fn mouse_event(&mut self, x: f32, y: f32, event: impl FnOnce(Point) -> Event) {
        self.pointer = to_screen(self.viewport, [x, y]);
        self.dispatch(event(self.pointer));
    }

    fn render_screen(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_image(ctx, self.screen.clone(), Color::BLACK);
        canvas.set_sampler(Sampler::nearest_clamp());
        let palette = self.state.palette;
        {
            let mut target = BatchedCanvas::new(&ctx.gfx, &mut canvas, SCREEN);
            let mut painter = Painter::new(&mut target, &palette);
            self.main.render(&self.state, SCREEN, &mut painter);
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
        if self.state.exit_requested {
            ctx.request_quit();
        }
        return Ok(());
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.render_screen(ctx)?;
        let mut frame = Canvas::from_frame(ctx, Color::BLACK);
        frame.set_sampler(Sampler::nearest_clamp());
        let WindowRect { x, y, w, h } = self.viewport;
        frame.draw(
            &self.screen,
            DrawParam::new()
                .dest([x, y])
                .scale([w / SCREEN_WIDTH as f32, h / SCREEN_HEIGHT as f32]),
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
        let event = Event::Key {
            key,
            mods: input.mods,
            repeat: repeated,
        };
        self.dispatch(event);
        return Ok(());
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
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
