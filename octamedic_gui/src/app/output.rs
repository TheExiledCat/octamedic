use octamedic_core::engine::handle::{OutputDevice, output_devices};
use rustuition::gadgets::dialog::{Checkbox, Dropdown};

use crate::{app::state::AppState, framework::widgets::Dialog};

// first in the list, before the devices
const DEFAULT_DEVICE: &str = "Default";
// device names can be long, like ALSA's plugins
const WIDTH: i32 = 560;
// what fits in the dropdown at WIDTH
const MAX_NAME_CHARS: usize = 50;
const CUT: &str = "...";

/// Which device plays and whether it's mixed to mono. OctaMED had Paula and nothing to pick.
pub fn open_output_dialog(state: &mut AppState) {
    let dialog = output_dialog(state, output_devices());
    state.open_dialog(dialog);
}

fn output_dialog(state: &AppState, devices: Vec<OutputDevice>) -> Dialog {
    let names: Vec<String> = [DEFAULT_DEVICE]
        .into_iter()
        .chain(devices.iter().map(|device| device.name.as_str()))
        .map(shortened)
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    // a device that's gone shows as the default
    let chosen = state
        .output_device()
        .and_then(|id| devices.iter().position(|device| device.id == id))
        .map_or(0, |index| index + 1);
    let mut dialog = Dialog::new("Output")
        .width(WIDTH)
        .submit_button("OK")
        .cancel_button("Cancel");
    let device = dialog.add_field(Dropdown::new("Device:", &names).chosen(chosen));
    let mono = dialog.add_field(Checkbox::new("Mix to mono").checked(state.mono()));
    return dialog.on_submit(move |state, result| {
        let picked = *result.get(device);
        // left alone, a device that's missing from the list stays
        if picked != chosen || state.audio_failed() {
            let id = picked
                .checked_sub(1)
                .and_then(|index| devices.get(index))
                .map(|device| device.id.clone());
            state.set_output_device(id);
        }
        state.set_mono(*result.get(mono));
    });
}

fn shortened(name: &str) -> String {
    if name.chars().count() <= MAX_NAME_CHARS {
        return name.to_owned();
    }
    let kept: String = name.chars().take(MAX_NAME_CHARS - CUT.len()).collect();
    return kept + CUT;
}

#[cfg(test)]
mod tests {
    use ggez::input::keyboard::{KeyCode, KeyMods};
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::gadget::Gadget;

    use super::*;
    use crate::framework::event::Event;

    fn devices() -> Vec<OutputDevice> {
        return ["Speakers", "Headphones"]
            .map(|name| OutputDevice {
                id: name.to_lowercase(),
                name: name.to_owned(),
            })
            .to_vec();
    }

    fn submit(dialog: &mut Dialog, state: &mut AppState) {
        let enter = Event::Key {
            key: KeyCode::Return,
            mods: KeyMods::NONE,
            repeat: false,
        };
        dialog.handle_event(state, &enter);
    }

    #[test]
    fn long_names_are_cut() {
        let name = "Plugin for channel downmix (stereo) with a simple spacialization";
        let short = shortened(name);
        assert_eq!(short.chars().count(), MAX_NAME_CHARS);
        assert!(short.ends_with(CUT));
        assert_eq!(shortened("Speakers"), "Speakers");
    }

    #[test]
    fn a_missing_device_stays_when_only_mono_changes() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.output_device = Some("unplugged".to_owned());
        let mut dialog = output_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert_eq!(state.output_device(), Some("unplugged"));
    }

    #[test]
    fn ok_keeps_what_was_picked() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_mono(true);
        let mut dialog = output_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert!(dialog.is_closed());
        assert_eq!(state.output_device(), None);
        assert!(state.mono());
    }
}
