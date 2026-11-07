use std::ops::Range;

pub use mmd::mmd::module::MAX_TRACKS;
use mmd::{
    mmd::module::{OctamedMMD0Block, OctamedMMD0BlockLine, OctamedMMD1Block},
    utility::bytes::{UByte, ValueMap},
};

use crate::data::{
    command::{CommandId, SET_VOLUME},
    instrument::InstrumentId,
    note::OctamedicNote,
    track_count::TrackCount,
    volume::{volume_argument, volume_from_argument},
};

pub const DEFAULT_LINES: usize = 64;

/// OctaMED only shows block counts/numbers as two digits, so there's never a 100th block.
pub const MAX_PATTERNS: usize = 100;

/// A block may have up to 3200 lines, per the manual.
pub const MAX_LINES: usize = 3200;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PatternId(pub u8);

impl PatternId {
    pub const ZERO: PatternId = PatternId(0);
}

impl From<UByte> for PatternId {
    fn from(value: UByte) -> Self {
        return Self(value.0);
    }
}

#[derive(Clone)]
pub struct OctamedicPattern {
    pub lines: Vec<OctamedicPatternLine>,
    pub name: String,
}

impl OctamedicPattern {
    pub fn new() -> Self {
        return Self {
            lines: vec![OctamedicPatternLine::new(TrackCount::default().tracks()); DEFAULT_LINES],
            name: String::new(),
        };
    }

    pub fn line_count(&self) -> usize {
        return self.lines.len();
    }

    pub fn track_count(&self) -> usize {
        return self.lines.first().map_or(0, |line| line.tracks.len());
    }

    pub fn track(&self, line: usize, track: usize) -> Option<&OctamedicPatternTrack> {
        return self.lines.get(line)?.tracks.get(track);
    }

    pub fn track_mut(&mut self, line: usize, track: usize) -> Option<&mut OctamedicPatternTrack> {
        return self.lines.get_mut(line)?.tracks.get_mut(track);
    }

    pub fn set_track_count(&mut self, count: usize) {
        let count = count.max(1);
        for line in &mut self.lines {
            line.tracks.resize(count, OctamedicPatternTrack::new());
        }
    }

    pub fn set_line_count(&mut self, count: usize) {
        let count = count.clamp(1, MAX_LINES);
        let track_count = self.track_count();
        self.lines
            .resize(count, OctamedicPatternLine::new(track_count));
    }

    /// Inserts an empty line at `index`, the Edit Bar's line, clamped into range.
    pub fn insert_line(&mut self, index: usize) {
        if self.lines.len() >= MAX_LINES {
            return;
        }
        let index = index.min(self.lines.len());
        let track_count = self.track_count();
        self.lines
            .insert(index, OctamedicPatternLine::new(track_count));
    }

    /// Deletes the line at `index`, unless it's the block's only one.
    pub fn delete_line(&mut self, index: usize) {
        if self.lines.len() > 1 && index < self.lines.len() {
            self.lines.remove(index);
        }
    }

    /// Takes `track`'s cell at `line` out: the cells below it in that track move up one and an
    /// empty one comes in at the bottom, so the block keeps its length.
    pub fn pull_track_up(&mut self, line: usize, track: usize) {
        if line >= self.lines.len() || track >= self.track_count() {
            return;
        }
        for index in line..self.lines.len() - 1 {
            self.lines[index].tracks[track] = self.lines[index + 1].tracks[track];
        }
        let last = self.lines.len() - 1;
        self.lines[last].tracks[track] = OctamedicPatternTrack::new();
    }

    /// The VOL slide: fills the lines between the volume commands around `line` in `track` with
    /// volume commands going evenly from the first's volume to the second's, rounded. `line` can
    /// be on either of them too. Returns whether there were two to slide between.
    pub fn slide_volume(&mut self, line: usize, track: usize, hex: bool) -> bool {
        if line >= self.lines.len() || track >= self.track_count() {
            return false;
        }
        let sets_volume =
            |index: &usize| self.lines[*index].tracks[track].command_id.0 == SET_VOLUME;
        let mut start = (0..=line).rev().find(sets_volume);
        let mut end = (line + 1..self.lines.len()).find(sets_volume);
        // on the last one, slide up to it
        if end.is_none() && start == Some(line) {
            end = start;
            start = (0..line).rev().find(sets_volume);
        }
        let (Some(start), Some(end)) = (start, end) else {
            return false;
        };
        let volume_at = |index: usize| {
            let argument = self.lines[index].tracks[track].command_value;
            return volume_from_argument(argument, hex) as f32;
        };
        let (from, to) = (volume_at(start), volume_at(end));
        let distance = (end - start) as f32;
        for (step, line) in self.lines[start + 1..end].iter_mut().enumerate() {
            let volume = from + (to - from) * (step + 1) as f32 / distance;
            let cell = &mut line.tracks[track];
            cell.command_id = CommandId(SET_VOLUME);
            cell.command_value = volume_argument(volume.round() as u8, hex);
        }
        return true;
    }

    /// Puts an empty cell in `track` at `line`: the cells below it in that track move down one and
    /// the bottom one drops off, so the block keeps its length.
    pub fn push_track_down(&mut self, line: usize, track: usize) {
        if line >= self.lines.len() || track >= self.track_count() {
            return;
        }
        for index in (line + 1..self.lines.len()).rev() {
            self.lines[index].tracks[track] = self.lines[index - 1].tracks[track];
        }
        self.lines[line].tracks[track] = OctamedicPatternTrack::new();
    }

    /// `track` as a block of its own, one track wide.
    pub fn copy_track(&self, track: usize) -> OctamedicPattern {
        let lines = self
            .lines
            .iter()
            .map(|line| OctamedicPatternLine {
                tracks: vec![line.tracks.get(track).copied().unwrap_or_default()],
                highlighted: false,
            })
            .collect();
        return OctamedicPattern {
            lines,
            name: String::new(),
        };
    }

    /// Overwrites `track` with `from`'s first track, for as many lines as both have.
    pub fn paste_track(&mut self, track: usize, from: &OctamedicPattern) {
        for (line, from) in self.lines.iter_mut().zip(&from.lines) {
            if let (Some(cell), Some(&from)) = (line.tracks.get_mut(track), from.tracks.first()) {
                *cell = from;
            }
        }
    }

    pub fn clear_track(&mut self, track: usize) {
        for line in &mut self.lines {
            if let Some(cell) = line.tracks.get_mut(track) {
                *cell = OctamedicPatternTrack::new();
            }
        }
    }

    /// Empties every cell, keeping the block's size and highlights.
    pub fn clear(&mut self) {
        for line in &mut self.lines {
            line.tracks.fill(OctamedicPatternTrack::new());
        }
    }

    /// The cells in `tracks` and `lines` as a block of their own, cut down to what this block
    /// has.
    pub fn copy_range(&self, tracks: Range<usize>, lines: Range<usize>) -> OctamedicPattern {
        let tracks = tracks.start.min(self.track_count())..tracks.end.min(self.track_count());
        let lines = lines.start.min(self.line_count())..lines.end.min(self.line_count());
        let lines = self.lines[lines]
            .iter()
            .map(|line| OctamedicPatternLine {
                tracks: line.tracks[tracks.clone()].to_vec(),
                highlighted: false,
            })
            .collect();
        return OctamedicPattern {
            lines,
            name: String::new(),
        };
    }

    /// Writes `from` over the block from `track` and `line`, right and down. Whatever runs off the
    /// block is left out.
    pub fn paste_range(&mut self, track: usize, line: usize, from: &OctamedicPattern) {
        for (to, from) in self.lines.iter_mut().skip(line).zip(&from.lines) {
            for (cell, &from) in to.tracks.iter_mut().skip(track).zip(&from.tracks) {
                *cell = from;
            }
        }
    }

    pub fn clear_range(&mut self, tracks: Range<usize>, lines: Range<usize>) {
        for line in self.lines.iter_mut().take(lines.end).skip(lines.start) {
            for cell in line.tracks.iter_mut().take(tracks.end).skip(tracks.start) {
                *cell = OctamedicPatternTrack::new();
            }
        }
    }

    /// Inserts an empty track at `index` in every line; the rightmost track is dropped, so the
    /// track count doesn't change.
    pub fn insert_track(&mut self, index: usize) {
        let track_count = self.track_count();
        for line in &mut self.lines {
            let index = index.min(line.tracks.len());
            line.tracks.insert(index, OctamedicPatternTrack::new());
            line.tracks.truncate(track_count);
        }
    }

    /// Deletes the track at `index` in every line, an empty one taking its place on the right.
    pub fn delete_track(&mut self, index: usize) {
        for line in &mut self.lines {
            if index < line.tracks.len() {
                line.tracks.remove(index);
                line.tracks.push(OctamedicPatternTrack::new());
            }
        }
    }

    /// Lengthens the block by inserting `factor - 1` empty lines after each line. Returns false,
    /// leaving the block alone, if that would take it past [`MAX_LINES`].
    pub fn expand(&mut self, factor: usize) -> bool {
        let factor = factor.max(1);
        let Some(length) = self.lines.len().checked_mul(factor) else {
            return false;
        };
        if length > MAX_LINES {
            return false;
        }
        let track_count = self.track_count();
        let mut lines = Vec::with_capacity(length);
        for line in self.lines.drain(..) {
            lines.push(line);
            lines.resize_with(lines.len() + factor - 1, || {
                OctamedicPatternLine::new(track_count)
            });
        }
        self.lines = lines;
        return true;
    }

    /// Keeps every `factor`th line, undoing [`Self::expand`].
    pub fn shrink(&mut self, factor: usize) {
        let factor = factor.max(1);
        self.lines = self.lines.iter().step_by(factor).cloned().collect();
        if self.lines.is_empty() {
            self.lines
                .push(OctamedicPatternLine::new(self.track_count()));
        }
    }

    /// Splits the lines from `index` onward into a new block, this one keeping what's before.
    /// `index` must be in `1..line_count()`, which only holds for a block with more than one line.
    pub fn split_at(&mut self, index: usize) -> OctamedicPattern {
        let index = index.clamp(1, self.lines.len() - 1);
        return OctamedicPattern {
            lines: self.lines.split_off(index),
            name: String::new(),
        };
    }

    /// Appends `other`'s lines, widening either block if their track counts differ. Returns
    /// false, leaving the block alone, if that would take it past [`MAX_LINES`].
    pub fn join(&mut self, other: &OctamedicPattern) -> bool {
        if self.lines.len() + other.lines.len() > MAX_LINES {
            return false;
        }
        let track_count = self.track_count().max(other.track_count());
        self.set_track_count(track_count);
        for line in &other.lines {
            let mut line = line.clone();
            line.tracks
                .resize(track_count, OctamedicPatternTrack::new());
            self.lines.push(line);
        }
        return true;
    }

    fn from_octamed(lines: &[OctamedMMD0BlockLine]) -> Self {
        let lines = lines
            .iter()
            .map(|line| OctamedicPatternLine {
                highlighted: false,
                tracks: line
                    .tracks
                    .iter()
                    .map(|t| OctamedicPatternTrack {
                        note: OctamedicNote::from_octamed(t.note_number),
                        instrument_id: t.instrument_number.remap(InstrumentId),
                        command_id: t.command_number.remap(CommandId),
                        command_value: t.command_value.0,
                    })
                    .collect(),
            })
            .collect();
        return Self {
            lines,
            name: String::new(),
        };
    }
}

impl Default for OctamedicPattern {
    fn default() -> Self {
        return Self::new();
    }
}

impl From<&OctamedMMD0Block> for OctamedicPattern {
    fn from(value: &OctamedMMD0Block) -> Self {
        return Self::from_octamed(&value.lines);
    }
}

impl From<&OctamedMMD1Block> for OctamedicPattern {
    fn from(value: &OctamedMMD1Block) -> Self {
        let mut pattern = Self::from_octamed(&value.lines);
        if let Some(info) = &value.info {
            pattern.name = info.block_name.clone();
            for (index, line) in pattern.lines.iter_mut().enumerate() {
                line.highlighted = info.highlight_mask.is_highlighted(index);
            }
        }
        return pattern;
    }
}

#[derive(Clone)]
pub struct OctamedicPatternLine {
    pub tracks: Vec<OctamedicPatternTrack>,
    pub highlighted: bool,
}

impl OctamedicPatternLine {
    fn new(track_count: usize) -> Self {
        return OctamedicPatternLine {
            tracks: vec![OctamedicPatternTrack::new(); track_count],
            highlighted: false,
        };
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OctamedicPatternTrack {
    pub note: OctamedicNote,
    pub instrument_id: InstrumentId,
    pub command_id: CommandId,
    pub command_value: u8,
}

impl OctamedicPatternTrack {
    pub fn new() -> Self {
        return Self {
            note: OctamedicNote::new(),
            instrument_id: InstrumentId(0),
            command_id: CommandId(0),
            command_value: 0,
        };
    }
}

impl Default for OctamedicPatternTrack {
    fn default() -> Self {
        return Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(lines: usize, tracks: usize) -> OctamedicPattern {
        let mut pattern = OctamedicPattern {
            lines: vec![OctamedicPatternLine::new(tracks); lines],
            name: String::new(),
        };
        for (index, line) in pattern.lines.iter_mut().enumerate() {
            line.tracks[0].note.value = index as u8 + 1;
        }
        return pattern;
    }

    fn notes(pattern: &OctamedicPattern, track: usize) -> Vec<u8> {
        return pattern
            .lines
            .iter()
            .map(|line| line.tracks[track].note.value)
            .collect();
    }

    #[test]
    fn pulling_a_track_up_keeps_the_length() {
        let mut pattern = numbered(4, 2);
        pattern.lines[2].tracks[1].note.value = 9;
        pattern.pull_track_up(1, 0);
        assert_eq!(notes(&pattern, 0), [1, 3, 4, 0]);
        // the other track stays put
        assert_eq!(notes(&pattern, 1), [0, 0, 9, 0]);
        pattern.pull_track_up(3, 0);
        assert_eq!(notes(&pattern, 0), [1, 3, 4, 0]);
    }

    #[test]
    fn pushing_a_track_down_drops_the_last() {
        let mut pattern = numbered(4, 2);
        pattern.push_track_down(1, 0);
        assert_eq!(notes(&pattern, 0), [1, 0, 2, 3]);
        assert_eq!(notes(&pattern, 1), [0, 0, 0, 0]);
        // past the block's edges nothing happens
        pattern.push_track_down(4, 0);
        pattern.push_track_down(0, 2);
        assert_eq!(notes(&pattern, 0), [1, 0, 2, 3]);
    }

    #[test]
    fn track_count_resizes_every_line() {
        let mut pattern = numbered(3, 2);
        pattern.set_track_count(4);
        assert!(pattern.lines.iter().all(|line| line.tracks.len() == 4));
        pattern.set_track_count(1);
        assert!(pattern.lines.iter().all(|line| line.tracks.len() == 1));
        // the kept track survives a shrink
        assert_eq!(pattern.lines[0].tracks[0].note.value, 1);
    }

    #[test]
    fn line_count_pads_or_cuts() {
        let mut pattern = numbered(3, 1);
        pattern.set_line_count(5);
        assert_eq!(pattern.line_count(), 5);
        assert_eq!(pattern.lines[3].tracks[0].note.value, 0);
        pattern.set_line_count(2);
        assert_eq!(pattern.line_count(), 2);
        assert_eq!(pattern.lines[1].tracks[0].note.value, 2);
    }

    #[test]
    fn line_count_caps_at_max_lines() {
        let mut pattern = numbered(1, 1);
        pattern.set_line_count(MAX_LINES + 100);
        assert_eq!(pattern.line_count(), MAX_LINES);
        pattern.insert_line(0);
        assert_eq!(pattern.line_count(), MAX_LINES);
    }

    #[test]
    fn insert_and_delete_line() {
        let mut pattern = numbered(3, 1);
        pattern.insert_line(1);
        assert_eq!(pattern.line_count(), 4);
        assert_eq!(pattern.lines[1].tracks[0].note.value, 0);
        assert_eq!(pattern.lines[2].tracks[0].note.value, 2);
        pattern.delete_line(1);
        assert_eq!(pattern.line_count(), 3);
        assert_eq!(pattern.lines[1].tracks[0].note.value, 2);
        // never below one line
        let mut single = numbered(1, 1);
        single.delete_line(0);
        assert_eq!(single.line_count(), 1);
    }

    #[test]
    fn insert_track_drops_the_rightmost() {
        let mut pattern = numbered(1, 3);
        for (index, track) in pattern.lines[0].tracks.iter_mut().enumerate() {
            track.note.value = index as u8 + 1;
        }
        pattern.insert_track(0);
        assert_eq!(pattern.track_count(), 3);
        assert_eq!(pattern.lines[0].tracks[0].note.value, 0);
        assert_eq!(pattern.lines[0].tracks[1].note.value, 1);
        // the old track 2 (value 3) was pushed off the end
        assert_eq!(pattern.lines[0].tracks[2].note.value, 2);
    }

    #[test]
    fn delete_track_backfills_from_the_right() {
        let mut pattern = numbered(1, 3);
        pattern.lines[0].tracks[1].note.value = 20;
        pattern.lines[0].tracks[2].note.value = 30;
        pattern.delete_track(1);
        assert_eq!(pattern.track_count(), 3);
        assert_eq!(pattern.lines[0].tracks[1].note.value, 30);
        assert_eq!(pattern.lines[0].tracks[2].note.value, 0);
    }

    #[test]
    fn expand_and_shrink_are_inverses() {
        let mut pattern = numbered(3, 1);
        pattern.expand(3);
        assert_eq!(pattern.line_count(), 9);
        let notes: Vec<u8> = pattern
            .lines
            .iter()
            .map(|l| l.tracks[0].note.value)
            .collect();
        assert_eq!(notes, [1, 0, 0, 2, 0, 0, 3, 0, 0]);
        pattern.shrink(3);
        assert_eq!(pattern.line_count(), 3);
        let notes: Vec<u8> = pattern
            .lines
            .iter()
            .map(|l| l.tracks[0].note.value)
            .collect();
        assert_eq!(notes, [1, 2, 3]);
    }

    #[test]
    fn split_and_join_round_trip() {
        let mut pattern = numbered(4, 1);
        let tail = pattern.split_at(2);
        assert_eq!(pattern.line_count(), 2);
        assert_eq!(tail.line_count(), 2);
        assert_eq!(tail.lines[0].tracks[0].note.value, 3);
        pattern.join(&tail);
        assert_eq!(pattern.line_count(), 4);
        let notes: Vec<u8> = pattern
            .lines
            .iter()
            .map(|l| l.tracks[0].note.value)
            .collect();
        assert_eq!(notes, [1, 2, 3, 4]);
    }

    #[test]
    fn copying_and_pasting_a_track() {
        let mut pattern = numbered(3, 2);
        let copied = pattern.copy_track(0);
        assert_eq!((copied.line_count(), copied.track_count()), (3, 1));
        assert_eq!(notes(&copied, 0), [1, 2, 3]);
        pattern.paste_track(1, &copied);
        assert_eq!(notes(&pattern, 1), [1, 2, 3]);
        pattern.clear_track(0);
        assert_eq!(notes(&pattern, 0), [0, 0, 0]);
        // a shorter buffer only overwrites the lines it has
        let mut longer = numbered(5, 1);
        longer.paste_track(0, &numbered(2, 1).copy_track(0));
        assert_eq!(notes(&longer, 0), [1, 2, 3, 4, 5]);
        longer.paste_track(0, &pattern.copy_track(0));
        assert_eq!(notes(&longer, 0), [0, 0, 0, 4, 5]);
    }

    #[test]
    fn clearing_keeps_size_and_highlights() {
        let mut pattern = numbered(3, 2);
        pattern.lines[1].highlighted = true;
        pattern.clear();
        assert_eq!((pattern.line_count(), pattern.track_count()), (3, 2));
        assert_eq!(notes(&pattern, 0), [0, 0, 0]);
        assert!(pattern.lines[1].highlighted);
    }

    #[test]
    fn expand_and_join_stop_at_max_lines() {
        let long = |lines| {
            let mut pattern = numbered(1, 1);
            pattern.set_line_count(lines);
            return pattern;
        };
        let mut pattern = long(MAX_LINES / 2);
        assert!(!pattern.expand(3));
        assert!(!pattern.expand(usize::MAX));
        assert_eq!(pattern.line_count(), MAX_LINES / 2);
        let other = long(MAX_LINES / 2 + 1);
        assert!(!pattern.join(&other));
        assert_eq!(pattern.line_count(), MAX_LINES / 2);
        assert!(pattern.expand(2));
        assert_eq!(pattern.line_count(), MAX_LINES);
    }

    #[test]
    fn join_widens_the_narrower_block() {
        let mut pattern = numbered(1, 1);
        let other = numbered(1, 3);
        pattern.join(&other);
        assert_eq!(pattern.track_count(), 3);
        assert_eq!(pattern.line_count(), 2);
    }

    #[test]
    fn volume_slide() {
        // the manual's example, in decimal
        let mut pattern = numbered(16, 2);
        let mut set_volume = |line: usize, argument: u8| {
            let cell = &mut pattern.lines[line].tracks[0];
            cell.command_id = CommandId(SET_VOLUME);
            cell.command_value = argument;
        };
        set_volume(9, 0x50);
        set_volume(14, 0x34);
        let arguments = |pattern: &OctamedicPattern| -> Vec<u8> {
            return (9..=14)
                .map(|line| pattern.lines[line].tracks[0].command_value)
                .collect();
        };
        let mut slid = pattern.clone();
        assert!(slid.slide_volume(11, 0, false));
        assert_eq!(arguments(&slid), [0x50, 0x47, 0x44, 0x40, 0x37, 0x34]);
        // notes stay, and the cursor can be on either end
        assert_eq!(slid.lines[11].tracks[0].note.value, 12);
        for line in [9, 14] {
            let mut on_an_end = pattern.clone();
            assert!(on_an_end.slide_volume(line, 0, false));
            assert_eq!(arguments(&on_an_end), arguments(&slid));
        }
        // in hex, 64 down to 32
        let mut hex = pattern.clone();
        hex.lines[9].tracks[0].command_value = 0x40;
        hex.lines[14].tracks[0].command_value = 0x20;
        assert!(hex.slide_volume(11, 0, true));
        assert_eq!(arguments(&hex), [0x40, 0x3A, 0x33, 0x2D, 0x26, 0x20]);
        // nothing to slide between
        assert!(!pattern.clone().slide_volume(11, 1, false));
        assert!(!pattern.clone().slide_volume(15, 0, false));
    }

    // track t, line l holds note 10 * t + l + 1, so where a cell came from shows
    fn grid(lines: usize, tracks: usize) -> OctamedicPattern {
        let mut pattern = OctamedicPattern {
            lines: vec![OctamedicPatternLine::new(tracks); lines],
            name: String::new(),
        };
        for (l, line) in pattern.lines.iter_mut().enumerate() {
            for (t, cell) in line.tracks.iter_mut().enumerate() {
                cell.note.value = (10 * t + l + 1) as u8;
            }
        }
        return pattern;
    }

    #[test]
    fn copying_a_range() {
        let range = grid(6, 4).copy_range(1..3, 2..4);
        assert_eq!((range.line_count(), range.track_count()), (2, 2));
        assert_eq!(notes(&range, 0), [13, 14]);
        assert_eq!(notes(&range, 1), [23, 24]);
        let edge = grid(6, 4).copy_range(3..9, 5..9);
        assert_eq!((edge.line_count(), edge.track_count()), (1, 1));
        assert_eq!(notes(&edge, 0), [36]);
    }

    #[test]
    fn pasting_a_range() {
        let range = grid(6, 4).copy_range(0..2, 0..2);
        let mut pattern = OctamedicPattern::new();
        pattern.set_line_count(4);
        pattern.paste_range(1, 1, &range);
        assert_eq!(notes(&pattern, 1), [0, 1, 2, 0]);
        assert_eq!(notes(&pattern, 2), [0, 11, 12, 0]);
        pattern.paste_range(3, 3, &range);
        assert_eq!(notes(&pattern, 3), [0, 0, 0, 1]);
    }

    #[test]
    fn clearing_a_range() {
        let mut pattern = grid(4, 3);
        pattern.clear_range(1..3, 1..3);
        assert_eq!(notes(&pattern, 0), [1, 2, 3, 4]);
        assert_eq!(notes(&pattern, 1), [11, 0, 0, 14]);
        assert_eq!(notes(&pattern, 2), [21, 0, 0, 24]);
    }
}
