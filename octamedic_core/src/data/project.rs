use std::sync::Arc;

use mmd::{
    mmd::{
        conversion::{FromModule, ToModule},
        module::{
            AMIGA_CHANNELS, BLOCK_INFO_RESERVED, EXPANSION_RESERVED, HEADER_RESERVED,
            INSTRUMENT_NAME_LENGTH, MMD1_ID, OctamedMMD, OctamedMMD0BlockLine, OctamedMMD0Color,
            OctamedMMD0ColorPalette, OctamedMMD0Dump, OctamedMMD0Expansion,
            OctamedMMD0ExpansionHeader, OctamedMMD0ExternalInstrument, OctamedMMD0Header,
            OctamedMMD0HeaderFlags, OctamedMMD0Info, OctamedMMD0InstrumentInfo,
            OctamedMMD0MidiCommands, OctamedMMD0NotationInfo, OctamedMMD0Rexx, OctamedMMD0Sample,
            OctamedMMD0SampleHeader, OctamedMMD0SampleTable, OctamedMMD0Song, OctamedMMD0SongFlags,
            OctamedMMD1Block, OctamedMMD1BlockHeader, OctamedMMD1BlockInfo,
            OctamedMMD1BlockInfoHeader, OctamedMMD1HighlightMask, OctamedMMD2BlockCommandPageTable,
            OctamedMMDBlockTable, OctamedMMDTrackLine, SEQUENCE_LENGTH,
        },
    },
    utility::bytes::{Byte, Offset, UByte, ULong, UWord, Word},
};

use crate::data::{
    instrument::{
        InstrumentKind, MAX_FINETUNE, MAX_HOLD_DECAY, MAX_INSTRUMENT_VOLUME, MAX_INSTRUMENTS,
        MIN_FINETUNE, OctamedicInstrument, OctamedicSample, OctamedicSynth, SampleOctaves,
    },
    notation::OctamedicNotation,
    palette::OctamedColorPalette,
    pattern::{OctamedicPattern, PatternId},
    song::{OctamedicSong, OctamedicSongFlags, SongId},
    tempo::OctamedicTempo,
    volume::OctamedicVolume,
};

pub struct OctamedicProject {
    name: String,
    songs: Vec<OctamedicSong>,
    palette: OctamedColorPalette,
    pub annotation: String,
    /// Channels split into two in 5-8 channel mode.
    pub split_channels: [bool; AMIGA_CHANNELS],
    pub notation: OctamedicNotation,
}

impl OctamedicProject {
    pub fn new() -> Self {
        return Self {
            name: String::new(),
            songs: vec![OctamedicSong::new("unnamed")],
            palette: OctamedColorPalette::DEFAULT,
            annotation: String::new(),
            split_channels: [true; AMIGA_CHANNELS],
            notation: OctamedicNotation::default(),
        };
    }

    pub fn get_song(&self, id: SongId) -> Option<&OctamedicSong> {
        return self.songs.get(id.0 as usize);
    }

    pub fn get_song_mut(&mut self, id: SongId) -> Option<&mut OctamedicSong> {
        return self.songs.get_mut(id.0 as usize);
    }

    pub fn songs(&self) -> &[OctamedicSong] {
        return &self.songs;
    }

    pub fn song_count(&self) -> usize {
        return self.songs.len();
    }

    pub fn palette(&self) -> OctamedColorPalette {
        return self.palette;
    }
}

impl Default for OctamedicProject {
    fn default() -> Self {
        return Self::new();
    }
}

impl FromModule for OctamedicProject {
    fn from_module(module: &mmd::mmd::module::OctamedMMD) -> Self {
        let mut s = Self {
            songs: vec![],
            ..Self::new()
        };
        if let Some(expansion) = &module.expansion_data {
            if !expansion.header.rgb_table_ptr.is_null() {
                s.palette = OctamedColorPalette::from_octamed(&expansion.color_palette);
            }
            s.annotation = expansion.annotation.clone();
            // a byte that isn't 0 means not split
            s.split_channels = expansion.header.channel_split.map(|byte| byte.0 == 0);
            if !expansion.header.notation_info_ptr.is_null() {
                s.notation = OctamedicNotation::from_octamed(&expansion.notation_info);
            }
        }
        s.name = module
            .expansion_data
            .as_ref()
            .map(|e| e.song_name.clone())
            .unwrap_or_default();
        let mut song = OctamedicSong::new(&s.name);
        let patterns: Vec<OctamedicPattern> = match &module.block_table {
            OctamedMMDBlockTable::MMD0BlockTable { headers: _, blocks } => {
                blocks.iter().map(OctamedicPattern::from).collect()
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers: _, blocks } => {
                blocks.iter().map(OctamedicPattern::from).collect()
            }
        };
        let patterns = patterns.into_iter().map(Arc::new).collect();
        let mmd_song = &module.song;
        let names = module.expansion_data.as_ref().map(|e| &e.instrument_infos);
        let extensions = module
            .expansion_data
            .as_ref()
            .map(|e| &e.external_instruments);
        for (index, slot) in song.instruments.iter_mut().enumerate() {
            let Some(Some(header)) = module.sample_table.headers.get(index) else {
                continue;
            };
            let info = &mmd_song.samples[index];
            let name = names
                .and_then(|names| names.get(index))
                // NUL padded
                .map_or(String::new(), |info| {
                    info.name.split('\0').next().unwrap_or_default().to_owned()
                });
            let mut instrument = OctamedicInstrument::new(name);
            instrument.volume = info.sample_volume.0.min(MAX_INSTRUMENT_VOLUME);
            instrument.transpose = info.sample_transpose.0;
            instrument.midi.channel = info.midi_channel.0;
            instrument.midi.preset = info.midi_preset.0;
            if let Some(extension) = extensions.and_then(|e| e.get(index)) {
                instrument.finetune = extension.fine_tune.0.clamp(MIN_FINETUNE, MAX_FINETUNE);
                instrument.hold = extension.hold.0.min(MAX_HOLD_DECAY);
                instrument.decay = extension.decay.0.min(MAX_HOLD_DECAY);
                instrument.midi.suppress_note_off = extension.supress_midi_off.0 != 0;
            }
            instrument.kind = instrument_kind(module, index, header, info);
            *slot = Some(instrument);
        }
        let song_length =
            (mmd_song.song_length.0 as usize).min(mmd_song.player_sequence_list.len());
        *song.sequence_mut() = mmd_song.player_sequence_list[..song_length]
            .iter()
            .map(|&id| PatternId::from(id))
            .collect();
        *song.tempo_mut() = OctamedicTempo::from_octamed(&mmd_song.get_tempo());
        let flags = &mmd_song.flags;
        song.flags = OctamedicSongFlags {
            filter_on: flags.filter_is_on(),
            volumes_hex: flags.volumes_are_hex(),
            st_slides: flags.use_st_sliding(),
            eight_channel: flags.is_8_channels(),
            jumping: flags.jumping_is_on(),
            jump_every_8th_line: flags.jump_every_eight_lines(),
            hq_v2_compatible: flags.is_hq_v2_compatability(),
            mixing: flags.mixing_enabled(),
        };
        song.global_transpose = mmd_song.global_transpose.0;
        song.master_volume = OctamedicVolume::new(mmd_song.master_volume.0);
        for (volume, mmd_volume) in song.track_volumes.iter_mut().zip(mmd_song.track_volumes) {
            *volume = OctamedicVolume::new(mmd_volume.0);
        }
        //single song for now
        song.patterns = patterns;
        s.songs = vec![song];
        return s;
    }
}

fn instrument_kind(
    module: &OctamedMMD,
    index: usize,
    header: &OctamedMMD0SampleHeader,
    info: &OctamedMMD0Sample,
) -> InstrumentKind {
    let table = &module.sample_table;
    if let Some(Some(synth)) = table.synths.get(index) {
        let synth_part = Arc::new(OctamedicSynth::from_octamed(synth));
        return match &synth.hybrid_sample {
            Some(bytes) => {
                let sample = OctamedicSample {
                    data: bytes.iter().map(|b| b.0).collect(),
                    repeat: None,
                    octaves: SampleOctaves::One,
                };
                InstrumentKind::Hybrid(synth_part, Arc::new(sample))
            }
            None => InstrumentKind::Synth(synth_part),
        };
    }
    let Some(Some(bytes)) = table.samples.get(index) else {
        return InstrumentKind::Empty;
    };
    let bytes: Vec<i8> = bytes.iter().map(|b| b.0).collect();
    return InstrumentKind::Sample(Arc::new(OctamedicSample::from_octamed(
        &bytes, header, info,
    )));
}

impl ToModule for OctamedicProject {
    // always a full MMD1 with its samples and expansion data embedded; MOD0/no-instrument
    // variants are a thin post-processing step on top of this, not this method's job
    fn to_module(&mut self) -> OctamedMMD {
        let song = self
            .get_song(SongId(0))
            .expect("a project always has a song");
        let block_table = block_table_to_octamed(song);
        let block_count = match &block_table {
            OctamedMMDBlockTable::MMD1BlockTable { headers, .. } => headers.len(),
            OctamedMMDBlockTable::MMD0BlockTable { headers, .. } => headers.len(),
        };
        let instruments = instrument_tables_to_octamed(song);
        let mmd_song = OctamedMMD0Song {
            samples: instruments.song_samples,
            block_count: UWord(block_count as u16),
            song_length: UWord(song.sequence().len().min(SEQUENCE_LENGTH) as u16),
            player_sequence_list: sequence_to_octamed(song),
            primary_tempo: UWord(song.tempo().primary_tempo),
            global_transpose: Byte(song.global_transpose),
            flags: song_flags_to_octamed(song),
            secondary_tempo: UByte(song.tempo().ticks_per_line),
            track_volumes: song.track_volumes.map(|volume| UByte(volume.get())),
            master_volume: UByte(song.master_volume.get()),
            sample_count: UByte(MAX_INSTRUMENTS as u8),
        };
        return OctamedMMD {
            header: header_to_octamed(),
            song: mmd_song,
            block_table,
            sample_table: instruments.sample_table,
            expansion_data: Some(expansion_to_octamed(
                self,
                song,
                instruments.instrument_infos,
                instruments.external_instruments,
            )),
        };
    }
}

fn header_to_octamed() -> OctamedMMD0Header {
    // module_length/song_ptr/block_array_ptr/sample_array_ptr/expansion_data_ptr are recomputed
    // by the writer from its own layout, so what's put here for them never matters
    return OctamedMMD0Header {
        id: ULong(MMD1_ID),
        module_length: ULong(0),
        song_ptr: Offset(0),
        player_seconds_num: UWord(0),
        player_sequence: UWord(0),
        block_array_ptr: Offset(0),
        flags: OctamedMMD0HeaderFlags {
            load_to_fast_memory: false,
        },
        reserved: [UByte(0); HEADER_RESERVED],
        sample_array_ptr: Offset(0),
        reserved2: ULong(0),
        expansion_data_ptr: Offset(0),
        reserved3: ULong(0),
        player_state: UWord(0),
        player_block: UWord(0),
        player_line: UWord(0),
        player_sequence_num: UWord(0),
        active_play_line: Word(-1),
        counter: UByte(0),
        extra_songs: UByte(0),
    };
}

fn sequence_to_octamed(song: &OctamedicSong) -> [UByte; SEQUENCE_LENGTH] {
    let mut sequence = [UByte(0); SEQUENCE_LENGTH];
    for (slot, id) in sequence.iter_mut().zip(song.sequence()) {
        *slot = UByte(id.0);
    }
    return sequence;
}

fn song_flags_to_octamed(song: &OctamedicSong) -> OctamedMMD0SongFlags {
    let flags = song.flags();
    let tempo = song.tempo();
    let mut byte1 = flags.filter_on as u8;
    byte1 |= (flags.jumping as u8) << 1;
    byte1 |= (flags.jump_every_8th_line as u8) << 2;
    byte1 |= (flags.volumes_hex as u8) << 4;
    byte1 |= (flags.st_slides as u8) << 5;
    byte1 |= (flags.eight_channel as u8) << 6;
    byte1 |= (flags.hq_v2_compatible as u8) << 7;
    // the file stores one less than the beat length
    let beat_length = tempo.lines_per_beat.clamp(1, 32) - 1;
    let mut byte2 = beat_length & 0x1f;
    byte2 |= (tempo.is_bpm_mode as u8) << 5;
    byte2 |= (flags.mixing as u8) << 7;
    return OctamedMMD0SongFlags::from_bytes(UByte(byte1), UByte(byte2));
}

fn block_table_to_octamed(song: &OctamedicSong) -> OctamedMMDBlockTable {
    let mut headers = Vec::with_capacity(song.patterns().len());
    let mut blocks = Vec::with_capacity(song.patterns().len());
    for pattern in song.patterns() {
        let track_count = pattern.track_count().max(1);
        let line_count = pattern.line_count().max(1);
        headers.push(OctamedMMD1BlockHeader {
            track_count: UWord(track_count as u16),
            // the file stores one less than the line count
            line_count: UWord((line_count - 1) as u16),
            info_ptr: Offset(0),
        });
        let lines = pattern
            .lines
            .iter()
            .map(|line| OctamedMMD0BlockLine {
                tracks: line
                    .tracks
                    .iter()
                    .map(|track| OctamedMMDTrackLine {
                        note_number: UByte(track.note.value),
                        instrument_number: UByte(track.instrument_id.0),
                        command_number: UByte(track.command_id.0),
                        command_value: UByte(track.command_value),
                    })
                    .collect(),
            })
            .collect();
        let info = OctamedMMD1BlockInfo {
            header: OctamedMMD1BlockInfoHeader {
                highlight_mask_array_ptr: Offset(0),
                block_name_string_ptr: Offset(0),
                block_name_length: ULong((pattern.name.chars().count() as u32) + 1),
                page_table_ptr: Offset(0),
                reserved: [ULong(0); BLOCK_INFO_RESERVED],
            },
            block_name: pattern.name.clone(),
            highlight_mask: OctamedMMD1HighlightMask::from_lines(
                pattern.lines.iter().map(|line| line.highlighted),
            ),
            page_table: OctamedMMD2BlockCommandPageTable {},
        };
        blocks.push(OctamedMMD1Block {
            lines,
            info: Some(info),
        });
    }
    return OctamedMMDBlockTable::MMD1BlockTable { headers, blocks };
}

struct InstrumentTables {
    song_samples: [OctamedMMD0Sample; MAX_INSTRUMENTS],
    sample_table: OctamedMMD0SampleTable,
    instrument_infos: Vec<OctamedMMD0InstrumentInfo>,
    external_instruments: Vec<OctamedMMD0ExternalInstrument>,
}

// always MAX_INSTRUMENTS slots, so index == InstrumentId.index() stays aligned with
// from_module's reading, whether or not that many instruments are actually in use
fn instrument_tables_to_octamed(song: &OctamedicSong) -> InstrumentTables {
    let mut song_samples = [OctamedMMD0Sample::new(); MAX_INSTRUMENTS];
    let mut headers = Vec::with_capacity(MAX_INSTRUMENTS);
    let mut samples = Vec::with_capacity(MAX_INSTRUMENTS);
    let mut synths = Vec::with_capacity(MAX_INSTRUMENTS);
    let mut instrument_infos = Vec::with_capacity(MAX_INSTRUMENTS);
    let mut external_instruments = Vec::with_capacity(MAX_INSTRUMENTS);
    for index in 0..MAX_INSTRUMENTS {
        let instrument = song.instruments[index].as_ref();
        instrument_infos.push(OctamedMMD0InstrumentInfo {
            name: padded_instrument_name(instrument.map_or("", |i| i.name.as_str())),
        });
        external_instruments.push(external_instrument_to_octamed(instrument));
        let Some(instrument) = instrument else {
            headers.push(None);
            samples.push(None);
            synths.push(None);
            continue;
        };
        let (repeat, repeat_length) = match instrument.sample() {
            Some(sample) => repeat_words_to_octamed(sample.repeat),
            None => (UWord(0), UWord(0)),
        };
        song_samples[index] = OctamedMMD0Sample {
            repeat,
            repeat_length,
            midi_channel: UByte(instrument.midi.channel),
            midi_preset: UByte(instrument.midi.preset),
            sample_volume: UByte(instrument.volume),
            sample_transpose: Byte(instrument.transpose),
        };
        // synths and hybrids can't be written back yet (see docs/FILES.md), so they're left
        // out the same as an empty instrument until that's implemented
        match &instrument.kind {
            InstrumentKind::Sample(sample) => {
                headers.push(Some(OctamedMMD0SampleHeader {
                    sample_length: ULong(sample.data.len() as u32),
                    sample_type: sample.octaves.to_octamed(),
                }));
                samples.push(Some(sample.data.iter().map(|&byte| Byte(byte)).collect()));
                synths.push(None);
            }
            InstrumentKind::Empty | InstrumentKind::Synth(_) | InstrumentKind::Hybrid(..) => {
                headers.push(None);
                samples.push(None);
                synths.push(None);
            }
        }
    }
    return InstrumentTables {
        song_samples,
        sample_table: OctamedMMD0SampleTable {
            headers,
            samples,
            synths,
        },
        instrument_infos,
        external_instruments,
    };
}

fn external_instrument_to_octamed(
    instrument: Option<&OctamedicInstrument>,
) -> OctamedMMD0ExternalInstrument {
    let Some(instrument) = instrument else {
        return OctamedMMD0ExternalInstrument {
            hold: UByte(0),
            decay: UByte(0),
            supress_midi_off: UByte(0),
            fine_tune: Byte(0),
            rest: vec![],
        };
    };
    return OctamedMMD0ExternalInstrument {
        hold: UByte(instrument.hold),
        decay: UByte(instrument.decay),
        supress_midi_off: UByte(instrument.midi.suppress_note_off as u8),
        fine_tune: Byte(instrument.finetune.clamp(MIN_FINETUNE, MAX_FINETUNE)),
        rest: vec![],
    };
}

// repeat/repeat_length are in words, and the in-memory sample is always 8-bit mono
fn repeat_words_to_octamed(repeat: Option<(usize, usize)>) -> (UWord, UWord) {
    let Some((start, end)) = repeat else {
        return (UWord(0), UWord(0));
    };
    return (UWord((start / 2) as u16), UWord(((end - start) / 2) as u16));
}

// NUL padded to INSTRUMENT_NAME_LENGTH chars; the writer appends one more terminator itself
fn padded_instrument_name(name: &str) -> String {
    let mut name: String = name.chars().take(INSTRUMENT_NAME_LENGTH).collect();
    while name.chars().count() < INSTRUMENT_NAME_LENGTH {
        name.push('\0');
    }
    return name;
}

fn notation_to_octamed(notation: &OctamedicNotation) -> OctamedMMD0NotationInfo {
    let mut flags = notation.flats as u8;
    flags |= (notation.three_four as u8) << 1;
    let pack_presets = |presets: [bool; crate::data::notation::NOTATION_PRESETS]| {
        return UByte(
            presets
                .iter()
                .enumerate()
                .fold(0u8, |bits, (preset, &on)| bits | ((on as u8) << preset)),
        );
    };
    return OctamedMMD0NotationInfo {
        sharp_count: UByte(notation.sharps),
        flags: UByte(flags),
        // -1 (0xffff) means none
        selected_tracks: notation
            .selected_tracks
            .map(|track| UWord(track.unwrap_or(0xffff))),
        shown_tracks: notation.shown_tracks.map(pack_presets),
        ghosted_tracks: notation.ghosted_tracks.map(pack_presets),
        note_transposes: notation.instrument_transposes.map(Byte),
        _pad: UByte(0),
    };
}

fn expansion_to_octamed(
    project: &OctamedicProject,
    song: &OctamedicSong,
    instrument_infos: Vec<OctamedMMD0InstrumentInfo>,
    external_instruments: Vec<OctamedMMD0ExternalInstrument>,
) -> OctamedMMD0Expansion {
    let channel_split = project
        .split_channels
        // a byte that isn't 0 means not split, see from_module
        .map(|split| UByte(if split { 0 } else { 1 }));
    let color_palette = OctamedMMD0ColorPalette {
        colors: std::array::from_fn(|pen| OctamedMMD0Color {
            value: UWord(project.palette().get(pen)),
        }),
    };
    let header = OctamedMMD0ExpansionHeader {
        next_module_ptr: Offset(0),
        expanded_instruments_array_ptr: Offset(0),
        expanded_instruments_array_length: UWord(external_instruments.len() as u16),
        extpanded_instruments_struct_size: UWord(OctamedMMD0ExternalInstrument::V4_SIZE as u16),
        annotation_text_char_array_ptr: Offset(0),
        annotation_text_length: ULong((project.annotation.len() as u32) + 1),
        instrument_info_ptr: Offset(0),
        instrument_info_array_length: UWord(instrument_infos.len() as u16),
        instrument_info_struct_size: UWord((INSTRUMENT_NAME_LENGTH + 1) as u16),
        jump_mask: ULong(0),
        // non-null so the palette always gets written
        rgb_table_ptr: Offset(1),
        channel_split,
        // the parser doesn't read notation info back yet (always OctamedMMD0NotationInfo::new()),
        // so writing it non-null would come back as all-zero instead of falling back to defaults
        notation_info_ptr: Offset(0),
        song_name_char_array_ptr: Offset(0),
        song_name_length: ULong((song.name().len() as u32) + 1),
        // not parsed yet, so nothing to carry over
        mmd_dump_ptr: Offset(0),
        mmd_info_ptr: Offset(0),
        mmd_rexx_ptr: Offset(0),
        mmd_midi_commands_ptr: Offset(0),
        reserved: [ULong(0); EXPANSION_RESERVED],
        tag_end: ULong(0),
    };
    return OctamedMMD0Expansion {
        header,
        external_instruments,
        annotation: project.annotation.clone(),
        instrument_infos,
        color_palette,
        notation_info: notation_to_octamed(&project.notation),
        song_name: song.name().to_owned(),
        mmd_dump: OctamedMMD0Dump {},
        mmd_info: OctamedMMD0Info {},
        mmd_rexx: OctamedMMD0Rexx {},
        mmd_midi_commands: OctamedMMD0MidiCommands {},
    };
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::Path};

    use mmd::mmd::{parser::OctamedMMDParser, writer::OctamedMMDWriter};

    use super::*;

    fn example(name: &str) -> OctamedicProject {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example_meds")
            .join(name);
        let module = OctamedMMDParser::new().parse_file(&path).unwrap().remove(0);
        return OctamedicProject::from_module(&module);
    }

    // loads `name`, writes it back out through to_module(), and reloads that
    fn round_trip(name: &str) -> (OctamedicProject, OctamedicProject) {
        let mut original = example(name);
        let module = original.to_module();
        let bytes = OctamedMMDWriter::new().write_module(&module).unwrap();
        let reparsed = OctamedMMDParser::new()
            .parse_module(&mut Cursor::new(bytes), Offset(0))
            .unwrap();
        let reloaded = OctamedicProject::from_module(&reparsed);
        return (original, reloaded);
    }

    #[test]
    fn song_flags() {
        let project = example("example.mmd0");
        let flags = project.get_song(SongId(0)).unwrap().flags();
        assert_eq!(
            flags,
            OctamedicSongFlags {
                eight_channel: true,
                ..OctamedicSongFlags::default()
            }
        );
    }

    #[test]
    fn finetune() {
        let project = example("example.mmd1");
        let song = project.get_song(SongId(0)).unwrap();
        let finetune = |index: usize| song.instruments[index].as_ref().unwrap().finetune;
        assert_eq!((finetune(0), finetune(1), finetune(8)), (-1, 0, 1));
    }

    #[test]
    fn round_trips_song_metadata() {
        for name in ["example.mmd0", "example.mmd1"] {
            let (original, reloaded) = round_trip(name);
            let original = original.get_song(SongId(0)).unwrap();
            let reloaded = reloaded.get_song(SongId(0)).unwrap();
            assert_eq!(reloaded.name(), original.name(), "{name}");
            assert_eq!(reloaded.flags(), original.flags(), "{name}");
            assert_eq!(reloaded.sequence(), original.sequence(), "{name}");
            assert_eq!(
                reloaded.global_transpose, original.global_transpose,
                "{name}"
            );
            assert_eq!(
                reloaded.master_volume.get(),
                original.master_volume.get(),
                "{name}"
            );
            assert_eq!(
                reloaded.track_volumes.map(|v| v.get()),
                original.track_volumes.map(|v| v.get()),
                "{name}"
            );
            let (a, b) = (original.tempo(), reloaded.tempo());
            assert_eq!(
                (
                    a.primary_tempo,
                    a.lines_per_beat,
                    a.ticks_per_line,
                    a.is_bpm_mode
                ),
                (
                    b.primary_tempo,
                    b.lines_per_beat,
                    b.ticks_per_line,
                    b.is_bpm_mode
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn round_trips_pattern_content() {
        for name in ["example.mmd0", "example.mmd1"] {
            let (original, reloaded) = round_trip(name);
            let original = original.get_song(SongId(0)).unwrap();
            let reloaded = reloaded.get_song(SongId(0)).unwrap();
            assert_eq!(
                reloaded.patterns().len(),
                original.patterns().len(),
                "{name}"
            );
            for (original, reloaded) in original.patterns().iter().zip(reloaded.patterns()) {
                assert_eq!(reloaded.line_count(), original.line_count(), "{name}");
                assert_eq!(reloaded.track_count(), original.track_count(), "{name}");
                let original_tracks = original.lines.iter().flat_map(|line| &line.tracks);
                let reloaded_tracks = reloaded.lines.iter().flat_map(|line| &line.tracks);
                for (original, reloaded) in original_tracks.zip(reloaded_tracks) {
                    assert_eq!(reloaded, original, "{name}");
                }
            }
        }
    }

    #[test]
    fn round_trips_instruments() {
        for name in ["example.mmd0", "example.mmd1"] {
            let (original, reloaded) = round_trip(name);
            let original = original.get_song(SongId(0)).unwrap();
            let reloaded = reloaded.get_song(SongId(0)).unwrap();
            for index in 0..MAX_INSTRUMENTS {
                let original = original.instruments[index].as_ref();
                let reloaded = reloaded.instruments[index].as_ref();
                assert_eq!(
                    reloaded.is_some(),
                    original.is_some(),
                    "{name} slot {index}"
                );
                let (Some(original), Some(reloaded)) = (original, reloaded) else {
                    continue;
                };
                assert_eq!(reloaded.name, original.name, "{name} slot {index}");
                assert_eq!(reloaded.volume, original.volume, "{name} slot {index}");
                assert_eq!(
                    reloaded.transpose, original.transpose,
                    "{name} slot {index}"
                );
                assert_eq!(reloaded.finetune, original.finetune, "{name} slot {index}");
                assert_eq!(reloaded.hold, original.hold, "{name} slot {index}");
                assert_eq!(reloaded.decay, original.decay, "{name} slot {index}");
                assert_eq!(reloaded.midi, original.midi, "{name} slot {index}");
                match (&original.kind, &reloaded.kind) {
                    (InstrumentKind::Sample(original), InstrumentKind::Sample(reloaded)) => {
                        assert_eq!(reloaded.data, original.data, "{name} slot {index}");
                        assert_eq!(reloaded.repeat, original.repeat, "{name} slot {index}");
                        assert_eq!(reloaded.octaves, original.octaves, "{name} slot {index}");
                    }
                    (InstrumentKind::Empty, InstrumentKind::Empty) => {}
                    // synths and hybrids aren't written back yet, so they come back empty
                    (
                        InstrumentKind::Synth(_) | InstrumentKind::Hybrid(..),
                        InstrumentKind::Empty,
                    ) => {}
                    (original, reloaded) => {
                        panic!("{name} slot {index}: {original:?} became {reloaded:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn round_trips_palette_and_annotation() {
        for name in ["example.mmd0", "example.mmd1"] {
            let (original, reloaded) = round_trip(name);
            assert_eq!(reloaded.palette(), original.palette(), "{name}");
            assert_eq!(reloaded.annotation, original.annotation, "{name}");
            assert_eq!(reloaded.split_channels, original.split_channels, "{name}");
        }
    }

    // notation_info_ptr is written null on purpose: the parser doesn't read notation info back
    // yet (see to_module's comment), so it always reloads as the default
    #[test]
    fn notation_does_not_round_trip_yet() {
        let (_, reloaded) = round_trip("example.mmd1");
        assert_eq!(reloaded.notation, OctamedicNotation::default());
    }
}
