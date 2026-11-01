use rustuition::gadgets::{button, slider};

use crate::{app::state::AppState, framework::event::Event};

pub mod audio_view;
pub mod symbols;

// rustuition's gadgets with the app's state filled in, so handlers know what they get
pub type Button = button::Button<AppState, Event>;
pub type Slider = slider::Slider<AppState, Event>;
