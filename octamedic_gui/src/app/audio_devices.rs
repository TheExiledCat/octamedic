use octamedic_core::devices::{AudioDevice, input_devices, output_devices};
use rustuition::gadgets::dialog::{Checkbox, Dropdown};

use crate::{
    app::state::AppState,
    framework::{event::Event, widgets::Dialog},
};

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

/// Which device the sample editor records from. OctaMED had the sampler on the parallel port.
pub fn open_input_dialog(state: &mut AppState) {
    let dialog = input_dialog(state, input_devices());
    state.open_dialog(dialog);
}

fn output_dialog(state: &AppState, devices: Vec<AudioDevice>) -> Dialog {
    let chosen = chosen_index(&devices, state.output_device());
    let mut dialog = devices_dialog("Output");
    let device = dialog.add_field(device_dropdown(&devices, chosen));
    let mono = dialog.add_field(Checkbox::new("Mix to mono").checked(state.mono()));
    return dialog.on_submit(move |state, result| {
        let picked = *result.get(device);
        // left alone, a device that's missing from the list stays
        let reopen =
            picked != chosen || state.audio_failed() || (state.using_fallback() && picked != 0);
        if reopen {
            state.set_output_device(picked_id(&devices, picked));
        }
        state.set_mono(*result.get(mono));
    });
}

// nothing has the input open, so picking one only saves it
fn input_dialog(state: &AppState, devices: Vec<AudioDevice>) -> Dialog {
    let chosen = chosen_index(&devices, state.input_device());
    let mut dialog = devices_dialog("Input");
    let device = dialog.add_field(device_dropdown(&devices, chosen));
    return dialog.on_submit(move |state, result| {
        let picked = *result.get(device);
        // left alone, a device that's missing from the list stays
        if picked != chosen {
            state.set_input_device(picked_id(&devices, picked));
        }
    });
}

fn devices_dialog(title: &str) -> Dialog {
    return Dialog::new(title)
        .width(WIDTH)
        .submit_button("OK")
        .cancel_button("Cancel");
}

fn device_dropdown(devices: &[AudioDevice], chosen: usize) -> Dropdown<AppState, Event> {
    let names: Vec<String> = [DEFAULT_DEVICE]
        .into_iter()
        .chain(devices.iter().map(|device| device.name.as_str()))
        .map(shortened)
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    return Dropdown::new("Device:", &names).chosen(chosen);
}

// a device that's gone shows as the default
fn chosen_index(devices: &[AudioDevice], saved: Option<&str>) -> usize {
    return saved
        .and_then(|id| devices.iter().position(|device| device.id == id))
        .map_or(0, |index| index + 1);
}

fn picked_id(devices: &[AudioDevice], picked: usize) -> Option<String> {
    return picked
        .checked_sub(1)
        .and_then(|index| devices.get(index))
        .map(|device| device.id.clone());
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

    fn devices() -> Vec<AudioDevice> {
        return ["Speakers", "Headphones"]
            .map(|name| AudioDevice {
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

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
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
    fn the_default_comes_before_the_devices() {
        let devices = devices();
        assert_eq!(chosen_index(&devices, None), 0);
        assert_eq!(chosen_index(&devices, Some("headphones")), 2);
        assert_eq!(chosen_index(&devices, Some("unplugged")), 0);
        assert_eq!(picked_id(&devices, 0), None);
        assert_eq!(picked_id(&devices, 2), Some("headphones".to_owned()));
    }

    #[test]
    fn a_missing_device_stays_when_only_mono_changes() {
        let mut state = state();
        state.config_mut().output.device = Some("unplugged".to_owned());
        let mut dialog = output_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert_eq!(state.output_device(), Some("unplugged"));
    }

    #[test]
    fn mono_with_active_fallback_keeps_saved_device() {
        let mut state = state();
        state.config_mut().output.device = Some("headphones".to_owned());
        state.set_using_fallback(true);
        let mut dialog = output_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert!(dialog.is_closed());
        assert_eq!(state.output_device(), Some("headphones"));
        assert!(state.using_fallback());
    }

    #[test]
    fn ok_keeps_what_was_picked() {
        let mut state = state();
        state.set_mono(true);
        let mut dialog = output_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert!(dialog.is_closed());
        assert_eq!(state.output_device(), None);
        assert!(state.mono());
    }

    #[test]
    fn a_missing_input_stays_when_left_alone() {
        let mut state = state();
        state.set_input_device(Some("unplugged".to_owned()));
        let mut dialog = input_dialog(&state, devices());
        submit(&mut dialog, &mut state);
        assert!(dialog.is_closed());
        assert_eq!(state.input_device(), Some("unplugged"));
    }

    #[test]
    fn the_same_input_again_leaves_the_config_alone() {
        let mut state = state();
        state.set_input_device(None);
        assert!(!state.config_changed());
        state.set_input_device(Some("headphones".to_owned()));
        assert!(state.config_changed());
        assert_eq!(state.input_device(), Some("headphones"));
    }
}
