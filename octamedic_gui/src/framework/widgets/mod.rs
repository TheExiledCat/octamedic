use rustuition::gadgets::{button, dialog, slider, stepped_slider, textbox};

use crate::{app::state::AppState, framework::event::Event};

pub mod audio_view;
pub mod symbols;
pub mod toggle_group;

pub use toggle_group::ToggleGroup;

// rustuition's gadgets with the app's state filled in, so handlers know what they get
pub type Button = button::Button<AppState, Event>;
pub type Dialog = dialog::Dialog<AppState, Event>;
pub type DialogFields<'a> = dialog::DialogFields<'a, AppState, Event>;
pub type Slider = slider::Slider<AppState, Event>;
pub type SteppedSlider = stepped_slider::SteppedSlider<AppState, Event>;
pub type Textbox = textbox::Textbox<AppState, Event>;
