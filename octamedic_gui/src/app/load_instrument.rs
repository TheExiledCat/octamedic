use std::{fs, path::Path};

use octamedic_core::{
    data::{
        instrument_loader::{InstrumentLoader, load_instrument},
        wav_loader::{WavChannel, WavInfo, WavLoader},
    },
    engine::period::note_period,
};
use rustuition::gadgets::dialog::{Checkbox, Cycle, Label};

use crate::{app::state::AppState, framework::widgets::Dialog};

// downsampling converts to the rate this note plays at, so it plays at its original pitch there.
// the numbers are the engine's notes, counted from 1 = C-1 in semitones, so 13 = C-2 and 25 = C-3
const NOTES: [(&str, u8); 3] = [("C-1", 1), ("C-2", 13), ("C-3", 25)];
const DEFAULT_NOTE: usize = 2;
const CHANNELS: [(&str, WavChannel); 3] = [
    ("Left", WavChannel::Left),
    ("Right", WavChannel::Right),
    ("Mixed", WavChannel::Mixed),
];
const DEFAULT_CHANNEL: usize = 2;

/// Asks how to load the file selected in the browser, then puts it in the active instrument.
pub fn load_selected_instrument(state: &mut AppState) {
    let Some(name) = state.browser.selected_file() else {
        state.notify("Select a sample to load first");
        return;
    };
    let path = state.browser.path().join(name);
    match wav_dialog(&path) {
        Ok(dialog) => state.open_dialog(dialog),
        Err(message) => state.notify(message),
    }
}

fn wav_dialog(path: &Path) -> Result<Dialog, String> {
    let any_wav = WavLoader {
        channel: WavChannel::Mixed,
        downsample_to: None,
        allow_aliasing: false,
    };
    let bytes = fs::read(path).map_err(|_| "Can't read file".to_owned())?;
    if !any_wav.recognizes(&bytes) {
        return Err("Not a sample".to_owned());
    }
    let info = WavLoader::info(&bytes).map_err(|error| error.to_string())?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let mut dialog = Dialog::new("Load instrument")
        .with(Label::new(&format!("{name}\n{}", describe(info))))
        .submit_button("Load")
        .cancel_button("Cancel");
    let downsample = dialog.add_field(Checkbox::new("Downsample").checked(true));
    let note_names = NOTES.map(|(name, _)| name);
    let note = dialog.add_field(Cycle::new("To note:", &note_names).chosen(DEFAULT_NOTE));
    // filled in by on_change
    let new_rate = dialog.add_field(Label::new(" "));
    let allow_aliasing = dialog.add_field(Checkbox::new("Allow aliasing"));
    let channel_names = CHANNELS.map(|(name, _)| name);
    let channel = (info.channels > 1)
        .then(|| dialog.add_field(Cycle::new("Channel:", &channel_names).chosen(DEFAULT_CHANNEL)));
    let path = path.to_path_buf();
    let dialog = dialog.on_change(move |state, fields| {
        let (_, target) = NOTES[fields.get(note)];
        let target = note_rate(state, target);
        let text = rate_text(fields.get(downsample), target, info.sample_rate);
        fields.set(new_rate, text);
    });
    return Ok(dialog.on_submit_checked(move |state, result| {
        let channel = channel.map_or(DEFAULT_CHANNEL, |field| *result.get(field));
        let (_, note) = NOTES[*result.get(note)];
        let loader = WavLoader {
            channel: CHANNELS[channel].1,
            downsample_to: result.get(downsample).then(|| note_rate(state, note)),
            allow_aliasing: *result.get(allow_aliasing),
        };
        let instrument = load_instrument(&path, &[&loader]).map_err(|error| error.to_string())?;
        state.replace_instrument(state.active_instrument, instrument);
        return Ok(());
    }));
}

// in Hz, on the machine it's running as
fn note_rate(state: &AppState, note: u8) -> u32 {
    return state.video().clock().paula_clock / note_period(note, 0, 0) as u32;
}

// what rate it ends up at, downsampling never goes up
fn rate_text(downsample: bool, target: u32, original: u32) -> String {
    if downsample && target < original {
        return format!("New rate: {target} Hz");
    }
    return format!("Stays at {original} Hz");
}

fn describe(info: WavInfo) -> String {
    let channels = match info.channels {
        1 => "mono".to_owned(),
        2 => "stereo".to_owned(),
        count => format!("{count} channels"),
    };
    return format!("{} Hz, {} bit, {channels}", info.sample_rate, info.bits);
}

#[cfg(test)]
mod tests {
    use ggez::input::keyboard::{KeyCode, KeyMods};
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::gadget::Gadget;

    use super::*;
    use crate::framework::event::Event;

    // a 16-bit PCM wav, `frames` of silence
    fn wav(channels: u16, sample_rate: u32, frames: u32) -> Vec<u8> {
        let block = channels * 2;
        let data_size = frames * block as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * block as u32).to_le_bytes());
        bytes.extend_from_slice(&block.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.resize(bytes.len() + data_size as usize, 0);
        return bytes;
    }

    fn state_with_file(name: &str, bytes: &[u8]) -> (AppState, std::path::PathBuf) {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let folder =
            std::env::temp_dir().join(format!("octamedic-wav-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(name), bytes).unwrap();
        state.browser.open(&folder).unwrap();
        state.browser.select_file(Some(name.to_owned()));
        return (state, folder);
    }

    fn press_enter(dialog: &mut Dialog, state: &mut AppState) {
        let enter = Event::Key {
            key: KeyCode::Return,
            mods: KeyMods::NONE,
            repeat: false,
        };
        dialog.handle_event(state, &enter);
    }

    #[test]
    fn needs_a_selection() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        load_selected_instrument(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("Select a sample to load first")
        );
    }

    #[test]
    fn other_files_arent_samples() {
        let (mut state, folder) = state_with_file("song.mmd0", b"not a sample");
        load_selected_instrument(&mut state);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(state.notification.as_deref(), Some("Not a sample"));
        assert!(state.take_pending_dialogs().is_empty());
    }

    #[test]
    fn broken_wav_says_so() {
        let (mut state, folder) = state_with_file("broken.wav", b"RIFF\x04\0\0\0WAVE");
        load_selected_instrument(&mut state);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(state.notification.as_deref(), Some("WAV file is damaged"));
    }

    #[test]
    fn wav_without_an_extension() {
        let (mut state, folder) = state_with_file("PAD", &wav(1, 8000, 100));
        load_selected_instrument(&mut state);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(state.take_pending_dialogs().len(), 1);
    }

    #[test]
    fn loads_into_the_active_instrument_at_c3() {
        let (mut state, folder) = state_with_file("pad.wav", &wav(2, 44_100, 44_100));
        state.set_instrument(3);
        load_selected_instrument(&mut state);
        let mut dialogs = state.take_pending_dialogs();
        assert_eq!(dialogs.len(), 1);
        press_enter(&mut dialogs[0], &mut state);
        let _ = fs::remove_dir_all(&folder);
        assert!(dialogs[0].is_closed());
        let instrument = state.song().instruments[2].clone().unwrap();
        assert_eq!(instrument.name, "pad.wav");
        let rate = note_rate(&state, 25) as usize;
        assert_eq!(instrument.sample.unwrap().data.len(), rate);
        assert!(!state.is_saved());
    }

    #[test]
    fn rate_text_says_where_it_ends_up() {
        assert_eq!(rate_text(true, 16_574, 44_100), "New rate: 16574 Hz");
        assert_eq!(rate_text(true, 16_574, 8000), "Stays at 8000 Hz");
        assert_eq!(rate_text(false, 16_574, 44_100), "Stays at 44100 Hz");
    }

    #[test]
    fn describes_the_file() {
        let info = WavInfo {
            sample_rate: 44_100,
            channels: 2,
            bits: 16,
        };
        assert_eq!(describe(info), "44100 Hz, 16 bit, stereo");
    }
}
