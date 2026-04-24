use crate::framework::widgets::title_bar::TitleBar;
use crate::framework::{input::InputEvent, widget::Widget, widgets::container::Container};
use ggez::conf::{FullscreenType, WindowMode};
use ggez::graphics::{DrawMode, DrawParam, FontData, Rect, Sampler};
use ggez::input::mouse;
use ggez::winit::dpi::LogicalSize;
use ggez::{
    event::EventHandler, graphics,
    graphics::{Canvas, Color},
    mint::Point2,
    Context,
    GameResult,
};
use taffy::{AvailableSpace, TaffyTree};

const TOPAZ_FONT: &[u8] = include_bytes!("../../assets/fonts/amiga-topaz.ttf");
pub const TOPAZ_FONT_KEY: &'static str = "Topaz";
pub const VIRTUAL_SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 640.0 * 2.0,
    h: 480. * 2.0,
};
pub struct App {
    root: Container,
    taffy: TaffyTree,
}

impl App {
    pub fn new(mut taffy: TaffyTree, ctx: &mut Context) -> GameResult<Self> {
        ctx.gfx
            .add_font(TOPAZ_FONT_KEY, FontData::from_slice(TOPAZ_FONT)?);
        let size = Self::get_largest_res(ctx);
        ctx.gfx.set_mode(WindowMode {
            fullscreen_type: FullscreenType::True,
            width: size.0 as f32,
            height: size.1 as f32,

            logical_size: Some(LogicalSize::new(VIRTUAL_SCREEN.w, VIRTUAL_SCREEN.h)),
            ..Default::default()
        })?;
        mouse::set_cursor_hidden(ctx, true);
        mouse::set_cursor_grabbed(ctx, true)?;

        let root = Self::root(&mut taffy);
        return Ok(Self { root, taffy });
    }
    fn get_largest_res(ctx: &Context) -> (u32, u32) {
        let mut largest_width = 0;
        for mode in ctx.gfx.window().current_monitor().unwrap().video_modes() {
            let size = mode.size();
            let width = size.width;
            let height = size.height;
            largest_width = largest_width.max(width);
        }
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

        let root = Container::new(taffy, vec![child]);

        return root;
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
        if ctx.mouse.delta() > Point2::from([0.0, 0.0]) {
            let pos = ctx.mouse.position();
            let del = ctx.mouse.delta();
            self.root.handle_event(
                &(InputEvent::MouseMove {
                    x: pos.x,
                    y: pos.y,
                    dx: del.x,
                    dy: del.y,
                }),
            );
        }

        return Ok(());
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::CYAN));
        canvas.set_sampler(Sampler::nearest_clamp());
        self.root.render(ctx, &mut canvas)?;
        let cursor = graphics::Mesh::new_circle(
            ctx,
            DrawMode::fill(),
            ctx.mouse.position(),
            25.0,
            0.1,
            Color::WHITE,
        )?;
        canvas.draw(&cursor, DrawParam::default());
        canvas.finish(ctx)?;
        return Ok(());
    }
}
