use rustuition::{
    gadget::Gadget,
    gadgets::grid::Grid,
    painter::{Align, Painter},
    rect::Rect,
};

use crate::{
    app::{
        fonts,
        header::labels::{Words, placed_at},
        mode::{HeaderMode, MainMode},
        state::AppState,
    },
    framework::{
        event::Event,
        widgets::{
            Button,
            symbols::{MusicSymbol, MusicSymbols, blocks},
        },
    },
};

const COLUMNS: usize = 3;
const ROWS: usize = 5;

const PICTURE_BLOCKS: i32 = 4;
// its Y sits a pixel closer than the font has it
const PLAY_LETTERS: Words = &[(4, "PLA"), (33, "Y")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeBinding {
    Header(HeaderMode),
    Main(MainMode),
    Both(HeaderMode, MainMode),
    // the play panel, and back to the last editor
    Play,
}

// in OctaMED's order; the picture buttons have no label
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

pub struct ModeButtons {
    grid: Grid<Button>,
}

impl ModeButtons {
    pub fn new() -> Self {
        let buttons = BUTTONS
            .iter()
            .map(|&(label, binding)| {
                let button = match binding {
                    ModeBinding::Main(MainMode::BlockEditor) => Button::new(blocks(PICTURE_BLOCKS)),
                    ModeBinding::Main(MainMode::NotationEditor) => Button::new(MusicSymbols([
                        MusicSymbol::Note,
                        MusicSymbol::BassClef,
                        MusicSymbol::Sharp,
                        MusicSymbol::HalfNote,
                    ])),
                    ModeBinding::Play => Button::new(placed_at(fonts::PANEL, 2, PLAY_LETTERS)),
                    _ => Button::new(label).align(Align::Left).font(fonts::PANEL),
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

impl Gadget<AppState, Event> for ModeButtons {
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
