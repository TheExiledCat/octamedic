use std::collections::HashSet;

use ggez::{
    Context, GameResult,
    conf::FullscreenType,
    event::{EventHandler, MouseButton},
    graphics,
    graphics::{Canvas, Color, DrawParam, FontData, Image, Rect, Sampler},
    input::mouse,
};
use taffy::{FlexDirection, Size, prelude::percent};

use crate::{
    app::{
        editor::{self, BlockEditor},
        footer, header, status_bar,
        title_bar::TitleBar,
    },
    framework::{
        input,
        ui::Ui,
        widgets::{label::TEXT_FONT, panel::Panel},
    },
};

const TOPAZ_FONT: &[u8] = include_bytes!("../../assets/fonts/amiga-topaz.ttf");

const OCTAMED_GUI: &[u8] = include_bytes!("../../assets/octamed.png");

const CURSOR: &[u8] = include_bytes!("../../assets/cursor.png");

pub const VIRTUAL_SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 800.,
    h: 600.,
};

pub const AMIGA_SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 640.,
    h: 201.,
};

pub struct App {
    ui: Ui,
    cursor: Image,
}

impl App {
    pub fn new(ctx: &mut Context) -> GameResult<Self> {

        ctx.gfx
            .add_font(TEXT_FONT, FontData::from_slice(TOPAZ_FONT)?);

        // ctx.gfx.set_fullscreen(FullscreenType::True)?;

        ctx.gfx
            .set_drawable_size(VIRTUAL_SCREEN.w, VIRTUAL_SCREEN.h)?;

        mouse::set_cursor_hidden(ctx, true);

        mouse::set_cursor_grabbed(ctx, true)?;

        let mut ui = Ui::new_styled(Panel::new(), |style| {

            style.flex_direction = FlexDirection::Column;

            style.size = Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            };
        });

        let root = ui.root();

        ui.add(root, TitleBar::new());

        header::build(&mut ui, root);

        status_bar::build(&mut ui, root);

        ui.add(root, BlockEditor::new());

        footer::build(&mut ui, root);

        return Ok(Self {
            ui,
            cursor: Image::from_bytes(ctx, CURSOR)?,
        });
    }

    fn get_largest_res(ctx: &Context) -> (u32, u32) {

        let mut largest_width = 0;

        let modes = ctx.gfx.window().current_monitor().unwrap().video_modes();

        let mut resolutions = vec![];

        for mode in modes {

            let size = mode.size();

            resolutions.push(size);

            let width = size.width;

            let height = size.height;

            largest_width = largest_width.max(width);
        }

        let resolutions = resolutions.into_iter().collect::<HashSet<_>>();

        let size = ctx
            .gfx
            .window()
            .current_monitor()
            .unwrap()
            .video_modes()
            .find(|m| m.size().width == largest_width)
            .map(|m| m.size())
            .unwrap();

        return (size.width, size.height);
    }

    fn draw_cursor(&self, ctx: &Context, canvas: &mut Canvas) {

        let scaling = VIRTUAL_SCREEN.h / AMIGA_SCREEN.h;

        let mut pos = ctx.mouse.position();

        pos.x -= pos.x % scaling;

        pos.y -= pos.y % scaling;

        canvas.draw(
            &self.cursor,
            DrawParam::default().dest(pos).scale([scaling, scaling]),
        );
    }

    fn forward_mouse_input(&mut self, ctx: &Context) {

        let pos = ctx.mouse.position();

        let pos = [pos.x, pos.y];

        self.ui.mouse_moved(pos);

        let buttons = [
            (MouseButton::Left, input::MouseButton::Left),
            (MouseButton::Right, input::MouseButton::Right),
        ];

        for (ggez_button, button) in buttons {

            if ctx.mouse.button_just_pressed(ggez_button) {

                self.ui.mouse_pressed(pos, button);
            }

            if ctx.mouse.button_just_released(ggez_button) {

                self.ui.mouse_released(pos, button);
            }
        }
    }
}

impl EventHandler for App {
    fn update(&mut self, ctx: &mut Context) -> GameResult {

        ctx.gfx
            .set_window_title(&format!("OctaMEDIC ({:.02} FPS) ", ctx.time.fps()));

        self.ui.update();

        let gfx: &Context = ctx;

        self.ui.compute_layout(
            Size {
                width: VIRTUAL_SCREEN.w,
                height: VIRTUAL_SCREEN.h,
            },
            |widget, known, available| widget.measure_content(gfx, known, available),
        );

        self.forward_mouse_input(ctx);

        // nothing above the widgets handles events yet; drain them so they don't pile up.
        self.ui.take_unhandled_events();

        return Ok(());
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {

        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));

        canvas.set_sampler(Sampler::nearest_clamp());

        self.ui.draw(&mut canvas);

        self.draw_cursor(ctx, &mut canvas);

        // {
        //     let img = Image::from_bytes(ctx, OCTAMED_GUI)?;
        //     let param = DrawParam::new().color(Color::hex("F009"));
        //     canvas.draw_stretched(&img, VIRTUAL_SCREEN, param);
        // }
        canvas.finish(ctx)?;

        return Ok(());
    }
}

pub trait DrawStretched {
    fn draw_stretched(&mut self, image: &graphics::Image, target: Rect, param: DrawParam);
}

impl DrawStretched for Canvas {
    fn draw_stretched(&mut self, image: &graphics::Image, target: Rect, param: DrawParam) {

        let img_w = image.width() as f32;

        let img_h = image.height() as f32;

        let scale_x = target.w / img_w;

        let scale_y = target.h / img_h;

        let param = param.dest([target.x, target.y]).scale([scale_x, scale_y]);

        self.draw(image, param);
    }
}
