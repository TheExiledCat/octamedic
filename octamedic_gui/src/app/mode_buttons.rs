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

const PICTURE_BLOCKS: i32 = 4;

/// What a mode button switches to: a header panel, a main screen, both, or PLAY's own. The
/// panels and screens themselves are in `app/header/` and `app/main/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeBinding {
    Header(HeaderMode),
    Main(MainMode),
    Both(HeaderMode, MainMode),
    /// The play panel, and back to the editor last shown.
    Play,
}

/// Left to right, then top to bottom, as in OctaMED; the pictures have no label.
const BUTTONS: [(&str, ModeBinding); COLUMNS * ROWS] = [
    (
        "FILES",
        ModeBinding::Both(HeaderMode::Files, MainMode::Files),
    ),
    ("MISC", ModeBinding::Header(HeaderMode::Misc)),
    ("", ModeBinding::Main(MainMode::BlockEditor)),
    ("PLAY", ModeBinding::Play),
    ("VOL", ModeBinding::Header(HeaderMode::Volume)),
    ("", ModeBinding::Main(MainMode::NotationEditor)),
    ("INSTR", ModeBinding::Header(HeaderMode::Instrument)),
    ("MIDI", ModeBinding::Main(MainMode::Midi)),
    ("SYNTH", ModeBinding::Main(MainMode::SynthEditor)),
    ("BLOCK", ModeBinding::Header(HeaderMode::Block)),
    ("TRANS", ModeBinding::Header(HeaderMode::Transpose)),
    ("SMPED", ModeBinding::Main(MainMode::SampleEditor)),
    ("EDIT", ModeBinding::Header(HeaderMode::Edit)),
    ("RANGE", ModeBinding::Header(HeaderMode::Range)),
    ("SLIST", ModeBinding::Main(MainMode::SongList)),
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
            .map(|&(label, binding)| {
                let button = match binding {
                    ModeBinding::Main(MainMode::BlockEditor) => {
                        Button::new(Blocks::new(PICTURE_BLOCKS))
                    }
                    ModeBinding::Main(MainMode::NotationEditor) => Button::new([
                        MusicSymbol::Note,
                        MusicSymbol::BassClef,
                        MusicSymbol::Sharp,
                        MusicSymbol::HalfNote,
                    ]),
                    _ => Button::new(label).align(Align::Left).font(Font::Panel),
                };
                button.on_click(move |state, _| {
                    let (header, main) = match binding {
                        ModeBinding::Header(header) => (header, state.main_mode()),
                        ModeBinding::Main(main) => (state.header_mode(), main),
                        ModeBinding::Both(header, main) => (header, main),
                        ModeBinding::Play => (HeaderMode::Play, state.last_editor()),
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
        for (button, (_, binding)) in self.grid.cells_mut().iter_mut().zip(BUTTONS) {
            let showing = match binding {
                ModeBinding::Header(mode) | ModeBinding::Both(mode, _) => {
                    state.header_mode() == mode
                }
                ModeBinding::Main(mode) => state.main_mode() == mode,
                ModeBinding::Play => state.header_mode() == HeaderMode::Play,
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
