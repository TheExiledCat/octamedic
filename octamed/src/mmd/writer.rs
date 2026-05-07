use std::{ collections::HashMap, fs::File, io::Write, iter, path::PathBuf };

use crate::{
    mmd::{
        conversion::{ BinarySize, BinaryWriter },
        module::{
            OctamedMMD,
            OctamedMMD0Block,
            OctamedMMD0BlockHeader,
            OctamedMMD0BlockLine,
            OctamedMMD1Block,
            OctamedMMD1BlockHeader,
            OctamedMMDBlockTable,
            OctamedMMDTrackLine,
        },
    },
    utility::bytes::{ Offset, UByte, ULong, ValueMap },
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
    pub fn write_module(&mut self, mmd: &OctamedMMD) -> Result<&Vec<u8>> {
        self.alloc_module(mmd)?;
        self.write(mmd)?;
        return Ok(&self.writer);
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

        todo!()
    }
    pub fn write_module_file(&mut self, path: PathBuf, mmd: &OctamedMMD) -> Result<()> {
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
                        self.layout.alloc(&i.header, i.header.get_size(mmd));

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

        for header in mmd.sample_table.headers {
            if let Some(h) = header {
                self.layout.alloc(&header, h.get_size(mmd));
            } else {
                self.layout.alloc(&header, size_of::<Offset>() as u32);
            }
        }
        self.layout.alloc(&mmd.sample_table.headers, mmd.sample_table.headers.get_size());
        todo!();
        return Ok(());
    }
    fn alloc_expansions(&mut self, mmd: &OctamedMMD) -> Result<()> {
        todo!()
    }

    fn write_header(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.push_size();
        let header = &mmd.header;
        let song = &mmd.song;
        let blocks = &mmd.block_table;
        let samples = &mmd.sample_table;
        let expansion_data = &mmd.expansion_data;
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
        self.writer.write_bytes(&self.layout.get(expansion_data))?;
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
        return match &mmd.block_table {
            OctamedMMDBlockTable::MMD0BlockTable { headers, blocks } => {
                self.write_blocks_mmd0(mmd, headers, blocks)
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
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
                //for now highlight mask not implemented
                self.writer.write_bytes(&vec![UByte(0); count])?;
                self.writer.write_bytes(&self.layout.get(&i.header.block_name_string_ptr))?;
                self.writer.write_bytes(&ULong((i.block_name.chars().count() as u32) + 1))?;
                self.writer.write_bytes(&self.layout.get(&i.page_table))?;
                self.writer.write_bytes(&i.header.reserved)?;
            }
        }

        return Ok(());
    }
}
