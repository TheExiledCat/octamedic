use crate::{
    app::{
        mode::{HeaderMode, MainMode},
        state::AppState,
    },
    framework::{
        event::Event,
        font::Font,
        painter::{Align, Painter},
        rect::Rect,
        widget::OctamedicWidget,
        widgets::{
            button::Button,
            grid::Grid,
            symbols::{Blocks, MusicSymbol},
        },
    },
};

const COLUMNS: usize = 3;
const ROWS: usize = 5;

/// What a button switches to.
#[derive(Clone, Copy)]
enum Target {
    Header(HeaderMode),
    Main(MainMode),
    Both(HeaderMode, MainMode),
    /// The play panel, and back to the editor last shown.
    Play,
}

/// Left to right, then top to bottom, as in OctaMED; the pictures have no label.
const BUTTONS: [(&str, Target); COLUMNS * ROWS] = [
    ("FILES", Target::Both(HeaderMode::Files, MainMode::Files)),
    ("MISC", Target::Header(HeaderMode::Misc)),
    ("", Target::Main(MainMode::BlockEditor)),
    ("PLAY", Target::Play),
    ("VOL", Target::Header(HeaderMode::Volume)),
    ("", Target::Main(MainMode::NotationEditor)),
    ("INSTR", Target::Header(HeaderMode::Instrument)),
    ("MIDI", Target::Main(MainMode::Midi)),
    ("SYNTH", Target::Main(MainMode::SynthEditor)),
    ("BLOCK", Target::Header(HeaderMode::Block)),
    ("TRACK", Target::Header(HeaderMode::Track)),
    ("SMPED", Target::Main(MainMode::SampleEditor)),
    ("EDIT", Target::Header(HeaderMode::Edit)),
    ("RANGE", Target::Header(HeaderMode::Range)),
    ("SLIST", Target::Main(MainMode::SongList)),
];

/// The buttons at the header's top right, which pick the header's panel and what fills the main
/// screen. The one showing is drawn switched on.
pub struct ModeButtons {
    grid: Grid<Button>,
}

impl ModeButtons {
    pub fn new() -> Self {
        let buttons = BUTTONS
            .iter()
            .map(|&(label, target)| {
                let button = match target {
                    Target::Main(MainMode::BlockEditor) => Button::new(Blocks::new(4)),
                    Target::Main(MainMode::NotationEditor) => Button::new([
                        MusicSymbol::Note,
                        MusicSymbol::BassClef,
                        MusicSymbol::Sharp,
                        MusicSymbol::HalfNote,
                    ]),
                    _ => Button::new(label).align(Align::Left).font(Font::Panel),
                };
                button.on_click(move |state, _| {
                    let (header, main) = match target {
                        Target::Header(header) => (header, state.main_mode()),
                        Target::Main(main) => (state.header_mode(), main),
                        Target::Both(header, main) => (header, main),
                        Target::Play => (HeaderMode::Play, state.last_editor()),
                    };
                    state.set_modes(header, main);
                })
            })
            .collect();
        return Self {
            grid: Grid::new(COLUMNS, ROWS, buttons),
        };
    }
}

impl OctamedicWidget for ModeButtons {
    fn update(&mut self, state: &mut AppState) {
        for (button, (_, target)) in self.grid.cells_mut().iter_mut().zip(BUTTONS) {
            let showing = match target {
                Target::Header(mode) | Target::Both(mode, _) => state.header_mode() == mode,
                Target::Main(mode) => state.main_mode() == mode,
                Target::Play => state.header_mode() == HeaderMode::Play,
            };
            button.set_toggled(showing);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.grid.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        self.grid.render(state, area, painter);
    }
}
