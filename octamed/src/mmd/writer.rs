use std::{ collections::HashMap, fs::File, io::Write, path::PathBuf };

use crate::{
    mmd::{ conversion::{ BinarySize, BinaryWriter }, module::{ OctamedMMD, OctamedMMDBlockTable } },
    utility::bytes::{ Offset, UByte },
};
struct AllocatorLayout {
    cursor: u32,
    positions: HashMap<*const (), Offset>,
}
impl AllocatorLayout {
    pub fn new() -> Self {
        Self { cursor: 0, positions: HashMap::new() }
    }
    pub fn alloc<T>(&mut self, obj: &T, size: u32) {
        self.cursor = Self::align_up(self.cursor, 2);
        self.positions.insert(obj as *const _ as *const (), Offset(self.cursor));
        self.cursor += size;
    }
    fn get<T>(&self, obj: &T) -> Offset {
        return self.positions[&(obj as *const _ as *const ())];
    }
    fn align_up(x: u32, align: u32) -> u32 {
        (x + align - 1) & !(align - 1)
    }
}
type Result<T> = std::io::Result<T>;
pub struct OctamedMMDWriter {
    layout: AllocatorLayout,
    writer: Vec<u8>,
}

impl OctamedMMDWriter {
    pub fn new() -> Self {
        Self { layout: AllocatorLayout::new(), writer: Vec::new() }
    }
    pub fn write_module(&mut self, mmd: &OctamedMMD) -> Result<&Vec<u8>> {
        self.alloc_module(mmd)?;
        self.write(mmd)?;
        return Ok(&self.writer);
    }
    fn write(&mut self, mmd: &OctamedMMD) -> Result<()> {
        self.write_header(mmd)?;
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

                    //todo the blockinfo and cmd page table
                    todo!();
                }
            }
        }
        todo!();

        return Ok(());
    }
    fn alloc_samples(&mut self, mmd: &OctamedMMD) -> Result<()> {
        todo!()
    }
    fn alloc_expansions(&mut self, mmd: &OctamedMMD) -> Result<()> {
        todo!()
    }

    fn write_header(&mut self, mmd: &OctamedMMD) -> Result<()> {
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

        return Ok(());
    }
}
