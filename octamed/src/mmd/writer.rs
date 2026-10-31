use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

use crate::{
    mmd::error::{MmdError, MmdResult},
    mmd::{
        conversion::{BinarySize, BinaryWriter},
        module::{
            OctamedMMD, OctamedMMD0Block, OctamedMMD0BlockHeader, OctamedMMD0Header,
            OctamedMMD0Song, OctamedMMD1Block, OctamedMMD1BlockHeader, OctamedMMDBlockTable,
            OctamedMMDTrackLine,
        },
    },
    utility::bytes::{Offset, UByte, ULong, UWord, ValueMap}, // UByte used in MMD0 track encoding
};

type Result<T> = MmdResult<T>;

pub struct OctamedMMDWriter {
    layout: AllocatorLayout,
    writer: Vec<u8>,
    byte_count_stack: Vec<usize>,
}

impl OctamedMMDWriter {
    pub fn new() -> Self {
        return Self {
            layout: AllocatorLayout::new(),
            writer: Vec::new(),
            byte_count_stack: vec![],
        };
    }

    pub fn write_module(mut self, mmd: &OctamedMMD) -> Result<Vec<u8>> {
        self.alloc_module(mmd)?;
        let allocated = self.layout.cursor;
        // the cursor is now the module's length, which write_header uses
        self.write(mmd)?;
        assert_eq!(allocated as usize, self.writer.len());
        return Ok(self.writer);
    }

    pub fn write_module_file(self, path: PathBuf, mmd: &OctamedMMD) -> Result<()> {
        let bytes = self.write_module(mmd)?;
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        return Ok(());
    }

    fn push_size(&mut self) {
        self.byte_count_stack.push(self.writer.len());
    }

    fn pop_size(&mut self) -> usize {
        let last_size = self.byte_count_stack.pop().unwrap();
        let current_size = self.writer.len();
        return current_size - last_size;
    }

    fn assert_size(&mut self, size: usize) {
        assert_eq!(self.pop_size(), size)
    }

    fn write(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.write_header(mmd)?;
        self.write_song(mmd)?;
        self.write_blocks(mmd)?;
        self.write_samples(mmd)?;
        self.write_expansion_data(mmd)?;
        return Ok(());
    }

    fn alloc_module(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.alloc_header(mmd)?;
        self.alloc_song(mmd)?;
        self.alloc_blocks(mmd)?;
        self.alloc_samples(mmd)?;
        return self.alloc_expansions(mmd);
    }

    fn alloc_header(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let header = &mmd.header;
        self.layout.alloc(Slot::Header, header.get_size(mmd));
        return Ok(());
    }

    fn alloc_song(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let song = &mmd.song;
        self.layout.alloc(Slot::Song, song.get_size(mmd));
        return Ok(());
    }

    fn alloc_blocks(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let block_table = &mmd.block_table;
        //table
        self.layout
            .alloc(Slot::BlockTable, mmd.block_table.get_size(mmd));
        //actual blocks
        match block_table {
            crate::mmd::module::OctamedMMDBlockTable::MMD0BlockTable { headers, blocks: _ } => {
                for (i, header) in headers.iter().enumerate() {
                    self.layout
                        .alloc(Slot::BlockHeader(i), header.get_size(mmd));
                    let (track_count, line_count) = (header.track_count, header.line_count);
                    self.layout.alloc(
                        Slot::BlockData(i),
                        (track_count.0 as u32)
                            * ((line_count.0 as u32) + 1)
                            * OctamedMMDTrackLine::MMD0_SIZE as u32,
                    );
                }
            }
            crate::mmd::module::OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
                for (i, header) in headers.iter().enumerate() {
                    self.layout
                        .alloc(Slot::BlockHeader(i), header.get_size(mmd));
                    let (track_count, line_count) = (header.track_count, header.line_count);
                    self.layout.alloc(
                        Slot::BlockData(i),
                        (track_count.0 as u32)
                            * ((line_count.0 as u32) + 1)
                            * OctamedMMDTrackLine::MMD1_SIZE as u32,
                    );
                    if let Some(info_block) = &blocks[i].info {
                        self.layout
                            .alloc(Slot::BlockInfo(i), info_block.header.get_size(mmd));
                        let bits_per_ulong = u32::BITS as usize;
                        let count = ((line_count.0 as usize) + 1).div_ceil(bits_per_ulong);
                        self.layout.alloc(
                            Slot::BlockHighlights(i),
                            (count * size_of::<ULong>()) as u32,
                        );
                        self.layout.alloc(
                            Slot::BlockName(i),
                            (info_block.block_name.chars().count() as u32) + 1, //\0
                        );
                        //page table is ignored as only mmd1 is supported, default to  null
                    }
                }
            }
        }
        return Ok(());
    }

    fn alloc_samples(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.layout
            .alloc(Slot::SampleTable, mmd.sample_table.get_size(mmd));
        for (i, header) in mmd.sample_table.headers.iter().enumerate() {
            if let Some(h) = header {
                self.layout.alloc(Slot::SampleHeader(i), h.get_size(mmd));
                if (h.sample_type as i16) < 0 {
                    return Err(MmdError::Unsupported("synth instruments"));
                }
                self.layout.alloc(
                    Slot::SampleData(i),
                    mmd.sample_table.samples[i].as_deref().unwrap().len() as u32,
                );
            }
        }
        return Ok(());
    }

    fn alloc_expansions(&mut self, mmd: &OctamedMMD) -> Result<()> {
        if let Some(e) = &mmd.expansion_data {
            self.layout.alloc(Slot::Expansion, e.header.get_size(mmd));
            //single only for now
            //self.layout.alloc(&e.header.next_module_ptr, size);
            // empty writes a null pointer and a count of 0 instead
            if !e.external_instruments.is_empty() {
                self.layout.alloc(
                    Slot::ExternalInstruments,
                    (e.external_instruments.len() as u32)
                        * (e.header.extpanded_instruments_struct_size.0 as u32),
                );
            }
            self.layout.alloc(
                Slot::Annotation,
                (e.annotation.len() as u32) + 1, // +1 for null terminator, matches IntoBytes for String
            );
            self.layout.alloc(
                Slot::InstrumentInfos,
                (e.header.instrument_info_struct_size.0 * e.header.instrument_info_array_length.0)
                    as u32,
            );
            if !e.header.rgb_table_ptr.is_null() {
                self.layout
                    .alloc(Slot::Palette, e.color_palette.get_size(mmd));
            }
            if !e.header.notation_info_ptr.is_null() {
                self.layout
                    .alloc(Slot::NotationInfo, e.notation_info.get_size(mmd));
            }
            self.layout.alloc(
                Slot::SongName,
                (e.song_name.len() as u32) + 1, // +1 for null terminator, matches IntoBytes for String
            );
            // todo: mmd_dump, info, rexx and midi are written as null pointers for now
        }
        return Ok(());
    }

    fn write_header(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.push_size();
        let header = &mmd.header;
        self.writer.write_bytes(&header.id)?;
        self.writer.write_bytes(&ULong(self.layout.cursor))?; // module_length = total alloc size
        self.writer.write_bytes(&self.layout.at(Slot::Song))?;
        self.writer.write_bytes(&header.player_seconds_num)?;
        self.writer.write_bytes(&header.player_sequence)?;
        self.writer.write_bytes(&self.layout.at(Slot::BlockTable))?;
        self.writer.write_bytes(&header.flags)?;
        self.writer.write_bytes(&header.reserved)?;
        self.writer
            .write_bytes(&self.layout.at(Slot::SampleTable))?;
        self.writer.write_bytes(&header.reserved2)?;
        if mmd.expansion_data.is_some() {
            self.writer.write_bytes(&self.layout.at(Slot::Expansion))?;
        } else {
            self.writer.write_bytes(&Offset(0))?; // spec: expdata is optional
        }
        self.writer.write_bytes(&header.reserved3)?;
        self.writer.write_bytes(&header.player_state)?;
        self.writer.write_bytes(&header.player_block)?;
        self.writer.write_bytes(&header.player_line)?;
        self.writer.write_bytes(&header.player_sequence_num)?;
        self.writer.write_bytes(&header.active_play_line)?;
        self.writer.write_bytes(&header.counter)?;
        self.writer.write_bytes(&header.extra_songs)?;
        self.assert_size(OctamedMMD0Header::SIZE);
        return Ok(());
    }

    fn write_song(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.push_size();
        let song = &mmd.song;
        for sample in &song.samples {
            self.writer.write_bytes(&sample.repeat)?;
            self.writer.write_bytes(&sample.repeat_length)?;
            self.writer.write_bytes(&sample.midi_channel)?;
            self.writer.write_bytes(&sample.midi_preset)?;
            self.writer.write_bytes(&sample.sample_volume)?;
            self.writer.write_bytes(&sample.sample_transpose)?;
        }
        self.writer.write_bytes(&song.block_count)?;
        self.writer.write_bytes(&song.song_length)?;
        self.writer.write_bytes(&song.player_sequence_list)?;
        self.writer.write_bytes(&song.primary_tempo)?;
        self.writer.write_bytes(&song.global_transpose)?;
        self.writer.write_bytes(&song.flags)?;
        self.writer.write_bytes(&song.secondary_tempo)?;
        self.writer.write_bytes(&song.track_volumes)?;
        self.writer.write_bytes(&song.master_volume)?;
        self.writer.write_bytes(&song.sample_count)?;
        self.assert_size(OctamedMMD0Song::SIZE);
        return Ok(());
    }

    fn write_blocks(&mut self, mmd: &OctamedMMD) -> Result<()> {
        //blocks
        return match &mmd.block_table {
            OctamedMMDBlockTable::MMD0BlockTable { headers, blocks } => {
                for i in 0..headers.len() {
                    self.writer
                        .write_bytes(&self.layout.at(Slot::BlockHeader(i)))?;
                }
                self.write_blocks_mmd0(mmd, headers, blocks)
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
                for i in 0..headers.len() {
                    self.writer
                        .write_bytes(&self.layout.at(Slot::BlockHeader(i)))?;
                }
                self.write_blocks_mmd1(headers, blocks)
            }
        };
    }

    fn write_blocks_mmd0(
        &mut self,
        _mmd: &OctamedMMD,
        headers: &Vec<OctamedMMD0BlockHeader>,
        blocks: &Vec<OctamedMMD0Block>,
    ) -> Result<()> {
        for (i, header) in headers.iter().enumerate() {
            self.writer.write_bytes(&header.track_count)?;
            self.writer.write_bytes(&header.line_count)?;
            let block = &blocks[i];
            for line in &block.lines {
                for track in &line.tracks {
                    let byte1 = {
                        UByte(
                            // note in bits 0-5, instrument's high bits in 6-7
                            track
                                .note_number
                                .map(|n| n & OctamedMMDTrackLine::BLOCK_LINE_NOTE_NUMBER_MASK_MMD0)
                                .0
                                | track
                                    .instrument_number
                                    .map(OctamedMMDTrackLine::instrument_high_bits_mmd0)
                                    .0,
                        )
                    };
                    let byte2 = {
                        UByte(
                            // command in bits 0-3, instrument's low bits in 4-7
                            track
                                .command_number
                                .map(|b| {
                                    b & OctamedMMDTrackLine::BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0
                                })
                                .0
                                | track
                                    .instrument_number
                                    .map(|i| i << OctamedMMDTrackLine::MMD0_INSTRUMENT_LOW_BITS)
                                    .0,
                        )
                    };
                    let byte3 = track.command_value;
                    self.writer.write_bytes(&byte1)?;
                    self.writer.write_bytes(&byte2)?;
                    self.writer.write_bytes(&byte3)?;
                }
            }
        }
        return Ok(());
    }

    fn write_blocks_mmd1(
        &mut self,
        headers: &Vec<OctamedMMD1BlockHeader>,
        blocks: &Vec<OctamedMMD1Block>,
    ) -> Result<()> {
        for (i, header) in headers.iter().enumerate() {
            self.writer.write_bytes(&header.track_count)?;
            self.writer.write_bytes(&header.line_count)?;
            let info_ptr = if blocks[i].info.is_some() {
                self.layout.at(Slot::BlockInfo(i))
            } else {
                Offset(0) // spec: info pointer "can be NULL"
            };
            self.writer.write_bytes(&info_ptr)?;
            let block = &blocks[i];
            for line in &block.lines {
                for track in &line.tracks {
                    let byte1 = {
                        track
                            .note_number
                            .map(|n| n & OctamedMMDTrackLine::BLOCK_LINE_NOTE_NUMBER_MASK_MMD1)
                    };
                    let byte2 = track
                        .instrument_number
                        .map(|i| i & OctamedMMDTrackLine::BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1);
                    let byte3 = track.command_number;
                    let byte4 = track.command_value;
                    self.writer.write_bytes(&byte1)?;
                    self.writer.write_bytes(&byte2)?;
                    self.writer.write_bytes(&byte3)?;
                    self.writer.write_bytes(&byte4)?;
                }
            }
            //blockinfo
            let info = &block.info;
            if let Some(info_blk) = info {
                let bits_per_ulong = u32::BITS as usize;
                let line_count = header.line_count;
                // line_count is zero-indexed; actual count is line_count.0 + 1
                let count = ((line_count.0 as usize) + 1).div_ceil(bits_per_ulong);
                self.writer
                    .write_bytes(&self.layout.at(Slot::BlockHighlights(i)))?;
                self.writer
                    .write_bytes(&self.layout.at(Slot::BlockName(i)))?;
                self.writer
                    .write_bytes(&ULong((info_blk.block_name.chars().count() as u32) + 1))?;
                // the page table is MMD2+ only
                self.writer.write_bytes(&Offset(0))?;
                self.writer.write_bytes(&info_blk.header.reserved)?;
                //for now highlight mask not implemented; write count ULongs of zeros
                self.writer.write_bytes(&vec![ULong(0); count])?;
                self.writer.write_bytes(&info_blk.block_name)?;
            }
        }
        return Ok(());
    }

    fn write_samples(&mut self, mmd: &OctamedMMD) -> Result<()> {
        //table
        for sample in 0..mmd.song.sample_count.0 {
            let header = &mmd.sample_table.headers[sample as usize];
            if header.is_some() {
                let ptr = self.layout.at(Slot::SampleHeader(sample as usize));
                self.writer.write_bytes(&ptr)?;
            } else {
                self.writer.write_bytes(&Offset(0))?;
            }
        }
        //samples
        for (i, header) in mmd.sample_table.headers.iter().enumerate() {
            if let Some(h) = header {
                let sample = mmd.sample_table.samples[i].as_deref().unwrap();
                self.writer.write_bytes(&h.sample_length)?;
                self.writer.write_bytes(&h.sample_type)?;
                self.writer.write_bytes(&sample)?;
            }
        }
        return Ok(());
    }

    fn write_expansion_data(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let exp = if let Some(e) = &mmd.expansion_data {
            e
        } else {
            return Ok(());
        };
        // next_module_ptr, null for single-song modules
        self.writer.write_bytes(&Offset(0))?;
        if exp.external_instruments.is_empty() {
            self.writer.write_bytes(&Offset(0))?;
            self.writer.write_bytes(&UWord(0))?;
            self.writer
                .write_bytes(&exp.header.extpanded_instruments_struct_size)?;
        } else {
            self.writer
                .write_bytes(&self.layout.at(Slot::ExternalInstruments))?;
            self.writer
                .write_bytes(&UWord(exp.external_instruments.len() as u16))?;
            self.writer
                .write_bytes(&exp.header.extpanded_instruments_struct_size)?;
        }
        self.writer.write_bytes(&self.layout.at(Slot::Annotation))?;
        self.writer
            .write_bytes(&exp.header.annotation_text_length)?;
        self.writer
            .write_bytes(&self.layout.at(Slot::InstrumentInfos))?;
        self.writer
            .write_bytes(&exp.header.instrument_info_array_length)?;
        self.writer
            .write_bytes(&exp.header.instrument_info_struct_size)?;
        self.writer.write_bytes(&exp.header.jump_mask)?;
        if exp.header.rgb_table_ptr.is_null() {
            self.writer.write_bytes(&Offset(0))?;
        } else {
            self.writer.write_bytes(&self.layout.at(Slot::Palette))?;
        }
        self.writer.write_bytes(&exp.header.channel_split)?;
        if exp.header.notation_info_ptr.is_null() {
            self.writer.write_bytes(&Offset(0))?;
        } else {
            self.writer
                .write_bytes(&self.layout.at(Slot::NotationInfo))?;
        }
        self.writer.write_bytes(&self.layout.at(Slot::SongName))?;
        self.writer.write_bytes(&exp.header.song_name_length)?;
        // not parsed yet, so null rather than a stale pointer from the original file
        self.writer.write_bytes(&Offset(0))?; // mmd_dump_ptr
        self.writer.write_bytes(&Offset(0))?; // mmd_info_ptr
        self.writer.write_bytes(&Offset(0))?; // mmd_rexx_ptr
        self.writer.write_bytes(&Offset(0))?; // mmd_midi_commands_ptr
        self.writer.write_bytes(&exp.header.reserved)?;
        self.writer.write_bytes(&exp.header.tag_end)?;
        //structs
        let size = exp.header.extpanded_instruments_struct_size.0 as usize;
        for instrument in &exp.external_instruments {
            let mut bytes = instrument.to_bytes();
            // entries shorter than v4's were read with the missing fields at 0
            bytes.resize(size, 0);
            for byte in bytes {
                self.writer.write_bytes(&UByte(byte))?;
            }
        }
        self.writer.write_bytes(&exp.annotation)?;
        for info in &exp.instrument_infos {
            self.writer.write_bytes(&info.name)?;
        }
        if !exp.header.rgb_table_ptr.is_null() {
            self.writer.write_bytes(&exp.color_palette)?;
        }
        if !exp.header.notation_info_ptr.is_null() {
            let info = &exp.notation_info;
            self.writer.write_bytes(&info.sharp_count)?;
            self.writer.write_bytes(&info.flags)?;
            self.writer.write_bytes(&info.selected_tracks)?;
            self.writer.write_bytes(&info.shown_tracks)?;
            self.writer.write_bytes(&info.ghosted_tracks)?;
            self.writer.write_bytes(&info.note_transposes)?;
            self.writer.write_bytes(&info._pad)?;
        }
        self.writer.write_bytes(&exp.song_name)?;
        return Ok(());
    }
}

impl Default for OctamedMMDWriter {
    fn default() -> Self {
        return Self::new();
    }
}

/// A part of the module file, placed by the layout before anything is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Slot {
    Header,
    Song,
    BlockTable,
    BlockHeader(usize),
    BlockData(usize),
    BlockInfo(usize),
    BlockHighlights(usize),
    BlockName(usize),
    SampleTable,
    SampleHeader(usize),
    SampleData(usize),
    Expansion,
    ExternalInstruments,
    Annotation,
    InstrumentInfos,
    Palette,
    NotationInfo,
    SongName,
}

struct AllocatorLayout {
    cursor: u32,
    positions: HashMap<Slot, Offset>,
}

impl AllocatorLayout {
    pub fn new() -> Self {
        return Self {
            cursor: 0,
            positions: HashMap::new(),
        };
    }

    pub fn alloc(&mut self, slot: Slot, size: u32) {
        self.positions.insert(slot, Offset(self.cursor));
        self.cursor += size;
    }

    /// Panics if `slot` wasn't allocated, which is a bug in the writer.
    fn at(&self, slot: Slot) -> Offset {
        return *self
            .positions
            .get(&slot)
            .unwrap_or_else(|| panic!("{slot:?} wasn't allocated"));
    }
}
