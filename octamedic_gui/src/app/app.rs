use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

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

#[cfg(target_os = "macos")]
use crate::macos;
use crate::{
    app::{
        config::Config, keypad, main_view::MainView, prompt, screen, state::AppState,
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
// the most a filter that bends the picture gets the screen enlarged each way. More than the window
// shows would only be squeezed out again
const MAX_FILTER_SCALE: i32 = 2;

pub struct App {
    state: AppState,
    main: MainView,
    dialogs: DialogStack,
    screen_presses: ScreenPresses,
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
    // the keypad never types notes, only into a box being typed in
    drop_keypad_char: bool,
    key_repeat: KeyRepeat,
    // the text event after a held back repeat gets dropped too
    repeat_held_back: bool,
    // the modifiers don't say which Alt, and right Alt is the Amiga key
    alts_held: [bool; 2],
}

impl App {
    /// `errors` are from starting up, before there was a window to show them in.
    pub fn new(
        ctx: &mut Context,
        project: OctamedicProject,
        config: Config,
        config_path: Option<PathBuf>,
        errors: Vec<String>,
    ) -> GameResult<Self> {
        mouse::set_cursor_hidden(ctx, true);
        // no audio device, run silent instead of bailing
        let mut state =
            AppState::with_config(project, EngineHandle::disconnected(), config, config_path);
        let audio = state.open_audio();
        let video = state.video();
        let mut app = Self {
            state,
            main: MainView::new(),
            dialogs: DialogStack::default(),
            screen_presses: ScreenPresses::default(),
            screen: screen_image(ctx, video),
            video,
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
            drop_keypad_char: false,
            key_repeat: KeyRepeat::default(),
            repeat_held_back: false,
            alts_held: [false; 2],
        };
        app.fit_viewport(ctx);
        app.state
            .report_startup_errors(errors.into_iter().chain(audio.err()).collect());
        return Ok(app);
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

    // Cmd+Q and the Dock's Quit go to the app delegate, not here, and it holds them back while
    // there are unsaved changes
    #[cfg(target_os = "macos")]
    fn ask_about_menu_quit(&mut self) {
        if macos::take_quit_request() {
            self.state.confirm_if_unsaved(AppState::request_exit);
        }
        macos::set_unsaved(!self.state.is_saved());
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
        return route_event(
            &mut self.state,
            &mut self.dialogs,
            &mut self.main,
            &mut self.screen_presses,
            &event,
        );
    }

    fn mouse_event(&mut self, ctx: &Context, x: f32, y: f32, event: impl FnOnce(Point) -> Event) {
        self.state.mouse_mods = self.with_amiga(ctx.keyboard.active_mods());
        self.pointer = screen(self.video).to_screen(self.viewport, [x, y]);
        self.dispatch(event(self.pointer));
    }

    // painted in memory instead of straight onto the GPU, so the filters get the finished pixels.
    // Curvature gets them enlarged as far as the window shows, so it doesn't drop whole rows and
    // columns
    fn filtered_screen(&mut self, ctx: &Context, area: Rect) -> Image {
        let mut frame = Framebuffer::new(area.w, area.h);
        {
            let mut painter = Painter::new(&mut frame, &self.state.palette);
            self.main.render(&self.state, area, &mut painter);
            self.dialogs.render(&self.state, area, &mut painter);
        }
        self.cursor_pixels.paint(&mut frame, self.pointer);
        let shown = [
            self.viewport.w / area.w as f32,
            self.viewport.h / area.h as f32,
        ];
        let finer = shown.map(|scale| (scale.floor() as i32).clamp(1, MAX_FILTER_SCALE));
        let frame = self.state.screen_filters.apply(frame, finer);
        return Image::from_pixels(
            ctx,
            &frame.to_rgba(),
            ImageFormat::Rgba8UnormSrgb,
            frame.width() as u32,
            frame.height() as u32,
        );
    }

    // the picture to show: the screen, or with filters on a bigger filtered copy of it
    fn render_screen(&mut self, ctx: &mut Context) -> GameResult<Image> {
        let area = screen(self.video).rect();
        if self.state.screen_filters.is_active() {
            return Ok(self.filtered_screen(ctx, area));
        }
        let mut canvas = Canvas::from_image(ctx, self.screen.clone(), Color::BLACK);
        canvas.set_sampler(Sampler::nearest_clamp());
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
        canvas.finish(ctx)?;
        return Ok(self.screen.clone());
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
        #[cfg(target_os = "macos")]
        self.ask_about_menu_quit();
        if self.state.exit_requested {
            self.state.flush_config();
            ctx.request_quit();
        }
        return Ok(());
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let picture = self.render_screen(ctx)?;
        let mut frame = Canvas::from_frame(ctx, Color::BLACK);
        frame.set_sampler(Sampler::nearest_clamp());
        let Viewport { x, y, w, h } = self.viewport;
        frame.draw(
            &picture,
            DrawParam::new()
                .dest([x, y])
                .scale([w / picture.width() as f32, h / picture.height() as f32]),
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
            if button == event::MouseButton::Right {
                self.state.right_button_held = true;
            }
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
            if button == event::MouseButton::Right {
                self.state.right_button_held = false;
            }
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

    // true cancels the quit
    fn quit_event(&mut self, _ctx: &mut Context) -> GameResult<bool> {
        return Ok(!quit_now(&mut self.state));
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
        self.drop_keypad_char = keypad::is_keypad(key) && !self.state.typing;
        self.dispatch(event);
        return Ok(());
    }

    fn key_up_event(&mut self, ctx: &mut Context, input: KeyInput) -> GameResult {
        if let Some(key) = input.keycode {
            self.key_repeat.released(key);
            self.track_alt(key, false);
            if !holding_other_keys(ctx, key) {
                self.state.end_chord();
            }
        }
        return Ok(());
    }

    fn text_input_event(&mut self, ctx: &mut Context, character: char) -> GameResult {
        // the letter of a key that answered a prompt isn't typed as well
        if self.repeat_held_back || self.prompt_took_key || self.drop_keypad_char {
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

fn holding_other_keys(ctx: &Context, released: KeyCode) -> bool {
    return ctx
        .keyboard
        .pressed_keys()
        .iter()
        .any(|&key| key != released && !is_modifier(key));
}

fn is_modifier(key: KeyCode) -> bool {
    return matches!(
        key,
        KeyCode::LShift
            | KeyCode::RShift
            | KeyCode::LControl
            | KeyCode::RControl
            | KeyCode::LAlt
            | KeyCode::RAlt
            | KeyCode::LWin
            | KeyCode::RWin
    );
}

// every quit ends up here: QUIT and Ctrl+Q once they've asked about unsaved changes, and the
// window's close button, which asks now
fn quit_now(state: &mut AppState) -> bool {
    if !state.exit_requested {
        state.confirm_if_unsaved(AppState::request_exit);
    }
    return state.exit_requested;
}

/// Mouse buttons whose press went to the screens rather than a prompt or dialog.
#[derive(Default)]
struct ScreenPresses {
    left: bool,
    right: bool,
}

impl ScreenPresses {
    fn held(&mut self, button: event::MouseButton) -> &mut bool {
        return match button {
            event::MouseButton::Left => &mut self.left,
            event::MouseButton::Right => &mut self.right,
        };
    }
}

// the prompt, then the top dialog, then the screens. a release goes back to the screens when they
// got its press, even if a prompt or dialog opened since, so whatever was pressed lets go. like a
// requester coming up ending the active gadget in Intuition
fn route_event(
    state: &mut AppState,
    dialogs: &mut DialogStack,
    screens: &mut impl Gadget<AppState, Event>,
    presses: &mut ScreenPresses,
    event: &Event,
) -> bool {
    match *event {
        Event::MouseDown { button, .. } => {
            *presses.held(button) = state.prompt.is_none() && dialogs.is_empty();
        }
        // forgetting the press either way
        Event::MouseUp { button, .. } if std::mem::take(presses.held(button)) => {
            return screens.handle_event(state, event);
        }
        _ => {}
    }
    return prompt::handle_event(state, event)
        || dialogs.handle_event(state, event)
        || screens.handle_event(state, event);
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
    use rustuition::rect::Rect;

    use super::*;
    use crate::app::{
        mode::{HeaderMode, MainMode},
        prompt::Prompt,
    };

    fn mouse(down: bool, x: i32, y: i32) -> Event {
        let (pos, button) = (Point::new(x, y), event::MouseButton::Left);
        return match down {
            true => Event::MouseDown { pos, button },
            false => Event::MouseUp { pos, button },
        };
    }

    // stands in for the screens, keeping what it gets
    #[derive(Default)]
    struct Screens {
        events: Vec<Event>,
    }

    impl Gadget<AppState, Event> for Screens {
        fn handle_event(&mut self, _state: &mut AppState, event: &Event) -> bool {
            self.events.push(*event);
            return true;
        }

        fn render(&mut self, _state: &AppState, _area: Rect, _painter: &mut Painter) {}
    }

    #[test]
    fn a_release_goes_where_its_press_went() {
        let mut state = state();
        let mut dialogs = DialogStack::default();
        let mut screens = Screens::default();
        let mut presses = ScreenPresses::default();
        let (press, release) = (mouse(true, 5, 5), mouse(false, 5, 5));
        let mut route = |state: &mut AppState, screens: &mut Screens, event: &Event| {
            route_event(state, &mut dialogs, screens, &mut presses, event);
        };
        route(&mut state, &mut screens, &press);
        // the press opened a prompt, which gets everything but that release
        state.open_prompt(Prompt::yes_no("Sure?", |_| {}));
        route(&mut state, &mut screens, &release);
        assert_eq!(screens.events, [press, release]);
        // and a press on the prompt keeps its release there
        route(&mut state, &mut screens, &press);
        route(&mut state, &mut screens, &release);
        assert_eq!(screens.events.len(), 2);
    }

    #[test]
    fn load_asking_first_doesnt_leave_the_list_scrolling() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-load-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        // more than the file list shows, so its scroll bar moves
        for index in 0..60 {
            std::fs::write(folder.join(format!("song{index:02}.mmd1")), []).unwrap();
        }
        let mut state = state();
        state.browser.open(&folder).unwrap();
        state.browser.select_file(Some("song00.mmd1".to_owned()));
        state.set_modes(HeaderMode::Files, MainMode::Files);
        state.set_song_name("changed");
        let mut dialogs = DialogStack::default();
        let mut main = MainView::new();
        let mut presses = ScreenPresses::default();
        let mut route = |state: &mut AppState, main: &mut MainView, event: Event| {
            route_event(state, &mut dialogs, main, &mut presses, &event);
            main.update(state);
        };
        let draw = |main: &mut MainView, state: &AppState| {
            let mut screen = Framebuffer::new(640, 256);
            main.render(
                state,
                screen.bounds(),
                &mut Painter::new(&mut screen, &state.palette),
            );
            return screen;
        };
        main.update(&mut state);
        let before = draw(&mut main, &state);
        // LOAD SONG asks about the changes, and the prompt is open when it's let go
        route(&mut state, &mut main, mouse(true, 170, 22));
        assert!(state.prompt.is_some());
        route(&mut state, &mut main, mouse(false, 170, 22));
        let no = Event::Key {
            key: KeyCode::N,
            mods: KeyMods::empty(),
            repeat: false,
        };
        route(&mut state, &mut main, no);
        assert!(state.prompt.is_none());
        // drag the file list's scroll bar down and let go
        route(&mut state, &mut main, mouse(true, 444, 105));
        route(
            &mut state,
            &mut main,
            Event::MouseMove {
                pos: Point::new(444, 150),
            },
        );
        route(&mut state, &mut main, mouse(false, 444, 150));
        let let_go = draw(&mut main, &state);
        // it's let go, so moving on doesn't scroll it any further
        route(
            &mut state,
            &mut main,
            Event::MouseMove {
                pos: Point::new(444, 190),
            },
        );
        let after = draw(&mut main, &state);
        let _ = std::fs::remove_dir_all(&folder);
        assert!(let_go != before, "the drag scrolled the list");
        assert!(after == let_go);
    }

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

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn answer(state: &mut AppState, key: KeyCode) {
        let event = Event::Key {
            key,
            mods: KeyMods::empty(),
            repeat: false,
        };
        assert!(prompt::handle_event(state, &event));
    }

    #[test]
    fn closing_a_saved_song_quits() {
        assert!(quit_now(&mut state()));
    }

    #[test]
    fn closing_with_unsaved_changes_asks_first() {
        let mut state = state();
        state.set_song_name("changed");
        assert!(!quit_now(&mut state));
        answer(&mut state, KeyCode::N);
        assert!(!quit_now(&mut state));
        answer(&mut state, KeyCode::Y);
        // the yes doesn't ask again
        assert!(quit_now(&mut state));
        assert!(state.prompt.is_none());
    }
}
