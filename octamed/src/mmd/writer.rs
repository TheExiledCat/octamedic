use std::{ collections::HashMap, fs::File, io::Write, iter, path::PathBuf };

use crate::{
    mmd::{
        conversion::{ BinarySize, BinaryWriter },
        module::{
            OctamedMMD,
            OctamedMMD0Block,
            OctamedMMD0BlockHeader,
            OctamedMMD0BlockLine,
            OctamedMMD0ExternalInstrument,
            OctamedMMD0InstrumentInfo,
            OctamedMMD0InstrumentType,
            OctamedMMD1Block,
            OctamedMMD1BlockHeader,
            OctamedMMDBlockTable,
            OctamedMMDTrackLine,
        },
    },
    utility::bytes::{ Offset, UByte, ULong, UWord, ValueMap },
};
struct AllocatorLayout {
    cursor: u32,
    positions: HashMap<*const (), Offset>,
}
impl AllocatorLayout {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            positions: HashMap::new(),
        }
    }
    pub fn alloc<T>(&mut self, obj: &T, size: u32) {
        self.cursor = Self::align_up(self.cursor, 2);
        self.positions.insert(obj as *const _ as *const (), Offset(self.cursor));
        self.cursor += size;
    }
    fn get<T>(&self, obj: &T) -> Offset {
        return self.positions
            .get(&(obj as *const _ as *const ()))
            .map(|o| *o)
            .unwrap_or(Offset(0));
    }
    fn align_up(x: u32, align: u32) -> u32 {
        (x + align - 1) & !(align - 1)
    }
}
type Result<T> = std::io::Result<T>;
pub struct OctamedMMDWriter {
    layout: AllocatorLayout,
    writer: Vec<u8>,
    byte_count_stack: Vec<usize>,
}

impl OctamedMMDWriter {
    pub fn new() -> Self {
        Self {
            layout: AllocatorLayout::new(),
            writer: Vec::new(),
            byte_count_stack: vec![],
        }
    }
    pub fn write_module(mut self, mmd: &OctamedMMD) -> Result<Vec<u8>> {
        self.alloc_module(mmd)?;
        self.write(mmd)?;
        return Ok(self.writer);
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
    pub fn write_module_file(self, path: PathBuf, mmd: &OctamedMMD) -> Result<()> {
        let bytes = self.write_module(mmd)?;
        let mut file = File::create(path)?;
        return file.write_all(&bytes);
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
        self.layout.alloc(header, header.get_size(mmd));

        return Ok(());
    }
    fn alloc_song(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let song = &mmd.song;
        self.layout.alloc(song, song.get_size(mmd));
        return Ok(());
    }
    fn alloc_blocks(&mut self, mmd: &OctamedMMD) -> Result<()> {
        let block_table = &mmd.block_table;
        //table
        self.layout.alloc(block_table, mmd.block_table.get_size(mmd));

        //actual blocks
        match block_table {
            crate::mmd::module::OctamedMMDBlockTable::MMD0BlockTable { headers, blocks } => {
                for (i, header) in headers.iter().enumerate() {
                    self.layout.alloc(header, header.get_size(mmd));
                    let (track_count, line_count) = (header.track_count, header.line_count);
                    self.layout.alloc(
                        &blocks[i],
                        (track_count.0 as u32) *
                            (line_count.0 as u32) *
                            ((size_of::<UByte>() as u32) * 3)
                    );
                }
            }
            crate::mmd::module::OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
                for (i, header) in headers.iter().enumerate() {
                    self.layout.alloc(header, header.get_size(mmd));
                    let (track_count, line_count) = (header.track_count, header.line_count);
                    self.layout.alloc(
                        &blocks[i],
                        (track_count.0 as u32) *
                            (line_count.0 as u32) *
                            ((size_of::<UByte>() as u32) * 4)
                    );

                    let info = &blocks[i].info;
                    if let Some(i) = info {
                        self.layout.alloc(&header.info_ptr, i.header.get_size(mmd));

                        let bits_per_ulong = size_of::<ULong>() * 8;

                        let count = ((line_count.0 as usize) + bits_per_ulong - 1) / bits_per_ulong;
                        self.layout.alloc(&i.header.highlight_mask_array_ptr, count as u32);

                        self.layout.alloc(
                            &i.header.block_name_string_ptr,
                            (i.block_name.chars().count() as u32) + 1 //\0
                        );
                        //page table is ignored as only mmd1 is supported, default to  null
                    }
                }
            }
        }

        return Ok(());
    }
    fn alloc_samples(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.layout.alloc(&mmd.sample_table, mmd.sample_table.get_size(mmd));

        for (i, header) in mmd.sample_table.headers.iter().enumerate() {
            if let Some(h) = header {
                self.layout.alloc(h, h.get_size(mmd));
                if (h.sample_type as i16) < 0 {
                    todo!("Synth instruments not implemented");
                }
                self.layout.alloc(
                    &mmd.sample_table.samples[i],
                    mmd.sample_table.samples[i].as_deref().unwrap().len() as u32
                );
            }
        }

        return Ok(());
    }
    fn alloc_expansions(&mut self, mmd: &OctamedMMD) -> Result<()> {
        if let Some(e) = &mmd.expansion_data {
            self.layout.alloc(&e.header, e.header.get_size(mmd));
            //single only for now
            //self.layout.alloc(&e.header.next_module_ptr, size);
            self.layout.alloc(
                &e.header.expanded_instruments_array_ptr,
                (e.header.extpanded_instruments_struct_size.0 *
                    e.header.expanded_instruments_array_length.0) as u32
            );

            self.layout.alloc(
                &e.header.annotation_text_char_array_ptr,
                e.header.annotation_text_length.0 as u32
            );
            self.layout.alloc(
                &e.header.instrument_info_ptr,
                (e.header.instrument_info_struct_size.0 *
                    e.header.instrument_info_array_length.0) as u32
            );
            self.layout.alloc(&e.header.rgb_table_ptr, e.color_pallete.get_size(mmd));
            self.layout.alloc(&e.header.notation_info_ptr, e.notation_info.get_size(mmd));
            self.layout.alloc(
                &e.header.song_name_char_array_ptr,
                e.header.song_name_length.0 as u32
            );

            self.layout.alloc(&e.header.mmd_dump_ptr, e.mmd_dump.get_size(mmd));
            self.layout.alloc(&e.header.mmd_info_ptr, e.mmd_info.get_size(mmd));
            self.layout.alloc(&e.header.mmd_rexx_ptr, e.mmd_rexx.get_size(mmd));
            self.layout.alloc(&e.header.mmd_midi_commands_ptr, e.mmd_midi_commands.get_size(mmd));
        }

        return Ok(());
    }

    fn write_header(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.push_size();
        let header = &mmd.header;
        let song = &mmd.song;
        let blocks = &mmd.block_table;
        let samples = &mmd.sample_table;

        self.writer.write_bytes(&header.id)?;
        self.writer.write_bytes(&header.module_length)?;
        self.writer.write_bytes(&self.layout.get(song))?;
        self.writer.write_bytes(&header.player_seconds_num)?;
        self.writer.write_bytes(&header.player_sequence)?;

        self.writer.write_bytes(&self.layout.get(blocks))?;
        self.writer.write_bytes(&header.flags)?;
        self.writer.write_bytes(&header.reserved)?;
        self.writer.write_bytes(&self.layout.get(samples))?;
        self.writer.write_bytes(&header.reserved2)?;
        if let Some(e) = &mmd.expansion_data {
            self.writer.write_bytes(&self.layout.get(&e.header))?;
        } else {
            self.writer.write_bytes(&Offset(0))?;
        }

        self.writer.write_bytes(&header.reserved3)?;
        self.writer.write_bytes(&header.player_state)?;
        self.writer.write_bytes(&header.player_block)?;
        self.writer.write_bytes(&header.player_line)?;
        self.writer.write_bytes(&header.player_sequence_num)?;
        self.writer.write_bytes(&header.active_play_line)?;
        self.writer.write_bytes(&header.counter)?;
        self.writer.write_bytes(&header.extra_songs)?;
        self.assert_size(52);
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
        self.assert_size(788);
        return Ok(());
    }
    fn write_blocks(&mut self, mmd: &OctamedMMD) -> Result<()> {
        //blocks
        return match &mmd.block_table {
            OctamedMMDBlockTable::MMD0BlockTable { headers, blocks } => {
                for (i, header) in headers.iter().enumerate() {
                    self.writer.write_bytes(&self.layout.get(header))?;
                }
                self.write_blocks_mmd0(mmd, headers, blocks)
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
                for (i, header) in headers.iter().enumerate() {
                    self.writer.write_bytes(&self.layout.get(header))?;
                }
                self.write_blocks_mmd1(mmd, headers, blocks)
            }
        };
    }
    fn write_blocks_mmd0(
        &mut self,
        mmd: &OctamedMMD,
        headers: &Vec<OctamedMMD0BlockHeader>,
        blocks: &Vec<OctamedMMD0Block>
    ) -> Result<()> {
        for (i, header) in headers.iter().enumerate() {
            self.writer.write_bytes(&header.track_count)?;
            self.writer.write_bytes(&header.line_count)?;
            let block = &blocks[i];
            for line in &block.lines {
                for track in &line.tracks {
                    let byte1 = {
                        UByte(
                            track.note_number.map(
                                |n| n & OctamedMMDTrackLine::BLOCK_LINE_NOTE_NUMBER_MASK_MMD0
                            ).0 & track.instrument_number.map(|i| (i >> 4) << 6).0
                        )
                    };
                    let byte2 = {
                        UByte(
                            track.command_number.map(|b| {
                                b & OctamedMMDTrackLine::BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0
                            }).0 & track.instrument_number.map(|i| i << 4).0
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
        mmd: &OctamedMMD,
        headers: &Vec<OctamedMMD1BlockHeader>,
        blocks: &Vec<OctamedMMD1Block>
    ) -> Result<()> {
        for (i, header) in headers.iter().enumerate() {
            self.writer.write_bytes(&header.track_count)?;
            self.writer.write_bytes(&header.line_count)?;
            self.writer.write_bytes(&self.layout.get(&header.info_ptr))?;
            let block = &blocks[i];
            for line in &block.lines {
                for track in &line.tracks {
                    let byte1 = {
                        track.note_number.map(
                            |n| n & OctamedMMDTrackLine::BLOCK_LINE_NOTE_NUMBER_MASK_MMD1
                        )
                    };
                    let byte2 = track.instrument_number.map(
                        |i| i & OctamedMMDTrackLine::BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1
                    );
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
            if let Some(i) = info {
                let bits_per_ulong = size_of::<ULong>() * 8;
                let line_count = header.line_count;
                let count = ((line_count.0 as usize) + bits_per_ulong - 1) / bits_per_ulong;
                //info header
                self.writer.write_bytes(&self.layout.get(&i.header.highlight_mask_array_ptr))?;
                self.writer.write_bytes(&self.layout.get(&i.header.block_name_string_ptr))?;
                self.writer.write_bytes(&ULong((i.block_name.chars().count() as u32) + 1))?;
                self.writer.write_bytes(&self.layout.get(&i.page_table))?;
                self.writer.write_bytes(&i.header.reserved)?;

                //for now highlight mask not implemented
                self.writer.write_bytes(&vec![UByte(0);count])?;
                self.writer.write_bytes(&i.block_name)?;
            }
        }

        return Ok(());
    }
    fn write_samples(&mut self, mmd: &OctamedMMD) -> Result<()> {
        //table
        for sample in 0..mmd.song.sample_count.0 {
            let header = &mmd.sample_table.headers[sample as usize];
            if let Some(h) = header {
                let ptr = self.layout.get(h);
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
        //header
        self.writer.write_bytes(&self.layout.get(&exp.header.next_module_ptr))?;

        self.writer.write_bytes(&self.layout.get(&exp.header.expanded_instruments_array_ptr))?;
        self.writer.write_bytes(&exp.header.expanded_instruments_array_length)?;
        self.writer.write_bytes(&exp.header.extpanded_instruments_struct_size)?;

        self.writer.write_bytes(&self.layout.get(&exp.header.annotation_text_char_array_ptr))?;
        self.writer.write_bytes(&exp.header.annotation_text_length)?;

        self.writer.write_bytes(&self.layout.get(&exp.header.instrument_info_ptr))?;
        self.writer.write_bytes(&exp.header.instrument_info_array_length)?;
        self.writer.write_bytes(&exp.header.instrument_info_struct_size)?;

        self.writer.write_bytes(&exp.header.jump_mask)?;
        self.writer.write_bytes(&self.layout.get(&exp.header.rgb_table_ptr))?;

        self.writer.write_bytes(&exp.header.channel_split)?;

        self.writer.write_bytes(&self.layout.get(&exp.header.notation_info_ptr))?;

        self.writer.write_bytes(&self.layout.get(&exp.header.song_name_char_array_ptr))?;
        self.writer.write_bytes(&exp.header.song_name_length)?;
        self.writer.write_bytes(&self.layout.get(&exp.header.mmd_dump_ptr))?;
        self.writer.write_bytes(&self.layout.get(&exp.header.mmd_info_ptr))?;
        self.writer.write_bytes(&self.layout.get(&exp.header.mmd_rexx_ptr))?;
        self.writer.write_bytes(&self.layout.get(&exp.header.mmd_midi_commands_ptr))?;
        self.writer.write_bytes(&exp.header.reserved)?;
        self.writer.write_bytes(&exp.header.tag_end)?;

        //structs
        //skip next mod again
        //self.writer.write_bytes(exp.next_mod)?;

        for inst in &exp.external_instruments {
            self.writer.write_bytes(&inst.hold)?;
            self.writer.write_bytes(&inst.decay)?;
            self.writer.write_bytes(&inst.supress_midi_off)?;
            self.writer.write_bytes(&inst.fine_tune)?;
        }
        self.writer.write_bytes(&exp.annotation)?;
        for info in &exp.instrument_infos {
            self.writer.write_bytes(&info.name)?;
        }
        self.writer.write_bytes(&exp.color_pallete)?;
        {
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
        //todo
        /* 
        self.writer.write_bytes(&exp.mmd_dump)?;
        self.writer.write_bytes(&exp.mmd_info)?;
        self.writer.write_bytes(&exp.mmd_rexx)?;
        self.writer.write_bytes(&exp.mmd_midi_commands)?;
        */

        return Ok(());
    }
}
