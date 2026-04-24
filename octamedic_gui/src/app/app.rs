use ggez::{ event::EventHandler, graphics::{ Canvas, Color }, mint::Point2 };
use taffy::{ AvailableSpace, TaffyTree };

use crate::framework::{ input::InputEvent, widget::Widget, widgets::container::Container };

const TOPAZ_FONT: &[u8] = include_bytes!("../../assets/fonts/amiga-topaz.ttf");

pub struct App {
    root: Container,
    taffy: TaffyTree,
}

impl App {
    pub fn new(mut taffy: TaffyTree) -> Self {
        let root = Self::root(&mut taffy);
        Self {
            root,
            taffy,
        }
    }
    fn root(taffy: &mut TaffyTree) -> Container {
        let child = Container::new(taffy, vec![]);
        let root = Container::new(taffy, vec![child.boxed()]);

        return root;
    }
}
impl EventHandler for App {
    fn update(&mut self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        self.taffy
            .compute_layout(self.root.core().node, taffy::Size {
                width: AvailableSpace::Definite(800.0),
                height: AvailableSpace::Definite(600.0),
            })
            .unwrap();
        self.root.layout(&self.taffy).unwrap();
        if ctx.mouse.delta() > Point2::from([0.0, 0.0]) {
            let pos = ctx.mouse.position();
            let del = ctx.mouse.delta();
            self.root.handle_event(
                &(InputEvent::MouseMove { x: pos.x, y: pos.y, dx: del.x, dy: del.y })
            );
        }

        return Ok(());
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));
        // self.root.render(ctx, &mut canvas)?;

        canvas.finish(ctx)?;
        return Ok(());
    }
}
