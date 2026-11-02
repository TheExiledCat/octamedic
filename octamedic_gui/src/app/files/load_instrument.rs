use std::{fs, rc::Rc};

use octamedic_core::{
    engine::period::note_period,
    import::{
        Decoded, InstrumentLoader, find_loader,
        flac::FlacLoader,
        instrument_name,
        mp3::Mp3Loader,
        msh::MshLoader,
        raw::RawLoader,
        sample::{DecodedSample, ImportSettings, SampleChannel},
        wav::WavLoader,
    },
};
use rustuition::gadgets::dialog::{Checkbox, Cycle, DialogResult, Field, Label, Toggle};

use crate::{
    app::state::AppState,
    framework::widgets::{Dialog, DialogFields},
};

// tried in order, raw takes anything so it's last
const LOADERS: &[&dyn InstrumentLoader] =
    &[&WavLoader, &FlacLoader, &Mp3Loader, &MshLoader, &RawLoader];

// downsampling converts to the rate this note plays at, so it plays at its original pitch there.
// the numbers are the engine's notes, counted from 1 = C-1 in semitones, so 13 = C-2 and 25 = C-3
const NOTES: [(&str, u8); 3] = [("C-1", 1), ("C-2", 13), ("C-3", 25)];
const DEFAULT_NOTE: usize = 2;
const CHANNELS: [(&str, SampleChannel); 3] = [
    ("Left", SampleChannel::Left),
    ("Right", SampleChannel::Right),
    ("Mixed", SampleChannel::Mixed),
];
const DEFAULT_CHANNEL: usize = 2;

/// Into the active instrument, asking how first if the format wants to.
pub fn load_selected_instrument(state: &mut AppState) {
    load_with(state, LOADERS);
}

fn load_with(state: &mut AppState, loaders: &[&dyn InstrumentLoader]) {
    let Some(name) = state.browser.selected_file() else {
        state.notify("Select a sample to load first");
        return;
    };
    let path = state.browser.path().join(name);
    let Ok(bytes) = fs::read(&path) else {
        state.notify("Can't read file");
        return;
    };
    let Some(loader) = find_loader(&bytes, loaders) else {
        state.notify("Not a sample");
        return;
    };
    let decoded = match loader.decode(&bytes) {
        Ok(decoded) => decoded,
        Err(error) => {
            state.notify(error.to_string());
            return;
        }
    };
    let name = instrument_name(&path);
    let decoded = match decoded {
        Decoded::Sample(sample) if loader.asks_for_settings() => {
            state.open_dialog(import_dialog(name, sample));
            return;
        }
        decoded => decoded,
    };
    match decoded.into_instrument(&name, ImportSettings::default()) {
        Ok(instrument) => state.replace_instrument(state.active_instrument, instrument),
        Err(error) => state.notify(error.to_string()),
    }
}

fn import_dialog(name: String, decoded: DecodedSample) -> Dialog {
    let mut dialog = Dialog::new("Load instrument")
        .with(Label::new(&format!("{name}\n{}", describe(&decoded))))
        .submit_button("Load")
        .cancel_button("Cancel");
    let downsample = dialog.add_field(Checkbox::new("Downsample").checked(true));
    let note_names = NOTES.map(|(name, _)| name);
    let note = dialog.add_field(Cycle::new("To note:", &note_names).chosen(DEFAULT_NOTE));
    // filled in by on_change
    let new_rate = dialog.add_field(Label::new(" "));
    let allow_aliasing = dialog.add_field(Checkbox::new("Allow aliasing"));
    let channel_names = CHANNELS.map(|(name, _)| name);
    let channel = (decoded.channels.len() > 1)
        .then(|| dialog.add_field(Cycle::new("Channel:", &channel_names).chosen(DEFAULT_CHANNEL)));
    let preview = dialog.add_field(Toggle::new("Preview", "Stop"));

    let decoded = Rc::new(decoded);
    let name = Rc::new(name);
    let original_rate = decoded.rate;
    let settings = move |state: &AppState, fields: &Fields| {
        let channel = channel.map_or(DEFAULT_CHANNEL, |field| fields.get(field));
        let (_, note) = NOTES[fields.get(note)];
        return ImportSettings {
            channel: CHANNELS[channel].1,
            downsample_to: fields.get(downsample).then(|| note_rate(state, note)),
            allow_aliasing: fields.get(allow_aliasing),
        };
    };

    // what's playing, so it only starts again when something changes
    let mut previewing = None;
    let (preview_sample, preview_name) = (decoded.clone(), name.clone());
    let dialog = dialog.on_change(move |state, fields| {
        let (_, target) = NOTES[fields.get(note)];
        let text = rate_text(
            fields.get(downsample),
            note_rate(state, target),
            original_rate,
        );
        fields.set(new_rate, text);

        let wanted = fields
            .get(preview)
            .then(|| (settings(state, &Fields::Open(fields)), target));
        if wanted == previewing {
            return;
        }
        previewing = wanted;
        let Some((settings, note)) = wanted else {
            state.stop_preview();
            return;
        };
        match preview_sample.to_instrument(&preview_name, settings) {
            Ok(instrument) => state.preview(&instrument, note),
            Err(error) => {
                state.notify(error.to_string());
                fields.set(preview, false);
                previewing = None;
            }
        }
    });
    return dialog
        .on_cancel(|state| state.stop_preview())
        .on_submit_checked(move |state, result| {
            state.stop_preview();
            let instrument = decoded
                .to_instrument(&name, settings(state, &Fields::Result(result)))
                .map_err(|error| error.to_string())?;
            state.replace_instrument(state.active_instrument, instrument);
            return Ok(());
        });
}

// the dialog's values while it's open, or once it's submitted
enum Fields<'a> {
    Open(&'a DialogFields<'a>),
    Result(&'a DialogResult),
}

impl Fields<'_> {
    fn get<T: Copy + 'static>(&self, field: Field<T>) -> T {
        return match self {
            Fields::Open(fields) => fields.get(field),
            Fields::Result(result) => *result.get(field),
        };
    }
}

fn note_rate(state: &AppState, note: u8) -> u32 {
    return state.video().clock().paula_clock / note_period(note, 0, 0) as u32;
}

fn rate_text(downsample: bool, target: u32, original: u32) -> String {
    if downsample && target < original {
        return format!("New rate: {target} Hz");
    }
    return format!("Stays at {original} Hz");
}

fn describe(decoded: &DecodedSample) -> String {
    let channels = match decoded.channels.len() {
        1 => "mono".to_owned(),
        2 => "stereo".to_owned(),
        count => format!("{count} channels"),
    };
    return match decoded.bits {
        Some(bits) => format!("{} Hz, {bits} bit, {channels}", decoded.rate),
        None => format!("{} Hz, {channels}", decoded.rate),
    };
}

#[cfg(test)]
mod tests {
    use ggez::input::keyboard::{KeyCode, KeyMods};
    use octamedic_core::{
        data::{instrument::InstrumentKind, project::OctamedicProject},
        engine::handle::EngineHandle,
    };
    use rustuition::gadget::Gadget;

    use super::*;
    use crate::framework::event::Event;

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
    fn synth_sounds_load_straight_away() {
        let mut msh = b"MSH\0".to_vec();
        msh.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1]);
        msh.extend_from_slice(&[0xff, 0xff]);
        msh.extend_from_slice(&28u32.to_be_bytes());
        msh.extend_from_slice(&[0, 1, 0x40, 0xc0]);
        let (mut state, folder) = state_with_file("SynthOrgan", &msh);
        load_selected_instrument(&mut state);
        let _ = fs::remove_dir_all(&folder);
        assert!(state.take_pending_dialogs().is_empty());
        let instrument = state.song().instruments[0].clone().unwrap();
        assert!(matches!(instrument.kind, InstrumentKind::Synth(_)));
        assert_eq!(instrument.data_bytes(), 2);
    }

    #[test]
    fn unrecognized_files_load_as_raw() {
        let (mut state, folder) = state_with_file("bassdrum", &[0x00, 0x7F, 0x80]);
        load_selected_instrument(&mut state);
        let _ = fs::remove_dir_all(&folder);
        assert!(state.take_pending_dialogs().is_empty());
        let instrument = state.song().instruments[0].clone().unwrap();
        assert_eq!(instrument.name, "bassdrum");
        assert_eq!(instrument.sample().unwrap().data, [0, 127, -128]);
    }

    #[test]
    fn nothing_recognizes_it_without_raw() {
        let (mut state, folder) = state_with_file("song.mmd0", b"not a wav");
        load_with(&mut state, &[&WavLoader]);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(state.notification.as_deref(), Some("Not a sample"));
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
        assert_eq!(instrument.sample().unwrap().data.len(), rate);
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
        let decoded = DecodedSample {
            rate: 44_100,
            bits: Some(16),
            channels: vec![vec![], vec![]],
            repeat: None,
        };
        assert_eq!(describe(&decoded), "44100 Hz, 16 bit, stereo");
        let mp3 = DecodedSample {
            bits: None,
            channels: vec![vec![]],
            ..decoded
        };
        assert_eq!(describe(&mp3), "44100 Hz, mono");
    }

    struct ReadyLoader;

    impl InstrumentLoader for ReadyLoader {
        fn recognizes(&self, bytes: &[u8]) -> bool {
            return bytes.starts_with(b"READY");
        }

        fn decode(
            &self,
            bytes: &[u8],
        ) -> Result<Decoded, octamedic_core::import::InstrumentLoadError> {
            return Ok(Decoded::Sample(DecodedSample {
                rate: 8287,
                bits: Some(8),
                channels: vec![bytes[5..].iter().map(|&b| b as i8 as f32 / 128.0).collect()],
                repeat: None,
            }));
        }

        fn asks_for_settings(&self) -> bool {
            return false;
        }
    }

    #[test]
    fn formats_without_settings_load_straight_away() {
        let (mut state, folder) = state_with_file("kick", b"READY\x10\x20\x30");
        load_with(&mut state, &[&ReadyLoader]);
        let _ = fs::remove_dir_all(&folder);
        assert!(state.take_pending_dialogs().is_empty());
        let instrument = state.song().instruments[0].clone().unwrap();
        assert_eq!(instrument.name, "kick");
        assert_eq!(instrument.sample().unwrap().data, [16, 32, 48]);
    }
}
