use crate::framework::widget::DefaultStyle;
use crate::framework::widgets::editor::Editor;
use crate::framework::widgets::footer::Footer;
use crate::framework::widgets::header::HeaderBar;
use crate::framework::widgets::status_bar::StatusBar;
use crate::framework::widgets::title_bar::TitleBar;
use crate::framework::{input, input::InputEvent, widget::Widget, widgets::container::Container};
use ggez::conf::FullscreenType;
use ggez::event::MouseButton;
use ggez::graphics::{DrawParam, FontData, Image, Rect, Sampler};
use ggez::input::mouse;
use ggez::{
    event::EventHandler, graphics,
    graphics::{Canvas, Color},
    mint::Point2,
    Context,
    GameResult,
};
use std::collections::HashSet;
use taffy::prelude::percent;
use taffy::{AvailableSpace, FlexDirection, Size, Style, TaffyTree};

const TOPAZ_FONT: &[u8] = include_bytes!("../../assets/fonts/amiga-topaz.ttf");
const OCTAMED_GUI: &[u8] = include_bytes!("../../assets/octamed.png");
const CURSOR: &[u8] = include_bytes!("../../assets/cursor.png");

pub const TOPAZ_FONT_KEY: &'static str = "Topaz";
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
    root: Container,
    taffy: TaffyTree,
}

impl App {
    pub fn new(mut taffy: TaffyTree, ctx: &mut Context) -> GameResult<Self> {
        ctx.gfx
            .add_font(TOPAZ_FONT_KEY, FontData::from_slice(TOPAZ_FONT)?);
        ctx.gfx.set_fullscreen(FullscreenType::True)?;
        ctx.gfx
            .set_drawable_size(VIRTUAL_SCREEN.w, VIRTUAL_SCREEN.h)?;

        mouse::set_cursor_hidden(ctx, true);
        mouse::set_cursor_grabbed(ctx, true)?;

        let root = Self::root(&mut taffy);
        return Ok(Self { root, taffy });
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
    fn root(taffy: &mut TaffyTree) -> Container {
        let child = TitleBar::new(taffy).boxed();
        let header = HeaderBar::new(taffy).boxed();
        let status_bar = StatusBar::new(taffy).boxed();
        let editor = Editor::new(taffy).boxed();
        let footer = Footer::new(taffy).boxed();
        let root = Container::new(taffy, vec![child, header, status_bar, editor, footer])
            .with_style(
                taffy,
                Style {
                    flex_direction: FlexDirection::Column,
                    size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    gap: Size::zero(),
                    ..Container::default_style()
                },
            );

        return root;
    }
    fn draw_cursor(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut Canvas,
    ) -> Result<(), ggez::GameError> {
        let image = Image::from_bytes(ctx, CURSOR)?;
        let height = VIRTUAL_SCREEN.h;
        let scaling = height / AMIGA_SCREEN.h;
        let mut pos = ctx.mouse.position();
        pos.x = pos.x - (pos.x % scaling);
        pos.y = pos.y - (pos.y % scaling);
        canvas.draw(
            &image,
            DrawParam::default().dest(pos).scale([scaling, scaling]),
        );
        return Ok(());
    }

    fn collect_events(&self, ctx: &mut Context) -> GameResult<Vec<InputEvent>> {
        let mut events = vec![];
        let pos = ctx.mouse.position();

        if ctx.mouse.delta() != Point2::from([0.0, 0.0]) {
            let delta = ctx.mouse.delta();
            events.push(InputEvent::MouseMove { pos, delta });
        }

        if ctx.mouse.button_just_pressed(MouseButton::Left) {
            events.push(InputEvent::MouseDown {
                pos,
                button: input::MouseButton::Left,
            })
        }
        if ctx.mouse.button_just_pressed(MouseButton::Right) {
            events.push(InputEvent::MouseDown {
                pos,
                button: input::MouseButton::Right,
            })
        }
        if ctx.mouse.button_just_released(MouseButton::Left) {
            events.push(InputEvent::MouseUp {
                pos,
                button: input::MouseButton::Left,
            })
        }
        if ctx.mouse.button_just_released(MouseButton::Right) {
            events.push(InputEvent::MouseUp {
                pos,
                button: input::MouseButton::Right,
            })
        }
        return Ok(events);
    }
}
impl EventHandler for App {
    fn update(&mut self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        ctx.gfx
            .set_window_title(&format!("OctaMEDIC ({:.02} FPS) ", ctx.time.fps()));
        self.root.update(ctx, &mut self.taffy)?;

        self.taffy
            .compute_layout(
                self.root.core().node,
                taffy::Size {
                    width: AvailableSpace::Definite(VIRTUAL_SCREEN.w),
                    height: AvailableSpace::Definite(VIRTUAL_SCREEN.h),
                },
            )
            .unwrap();
        self.root.layout(&self.taffy).unwrap();
        let events = self.collect_events(ctx)?;
        for event in events {
            self.root.handle_event(&event);
        }
        let signals = self.root.collect_signals();
        self.root.handle_signals(&signals);

        return Ok(());
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));
        canvas.set_sampler(Sampler::nearest_clamp());
        self.root.render(ctx, &mut canvas)?;
        self.draw_cursor(ctx, &mut canvas)?;

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
