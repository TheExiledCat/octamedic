use std::ops::Range;

use octamedic_core::data::pattern::PatternId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub track: usize,
    pub line: usize,
}

/// The RANGE panel's range: a rectangle of cells in one block. `anchor` is where it was started
/// and `end` where it's been taken to, so either can be any corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockRange {
    pub block: PatternId,
    pub anchor: Cell,
    pub end: Cell,
}

impl BlockRange {
    pub fn at(block: PatternId, cell: Cell) -> Self {
        return Self {
            block,
            anchor: cell,
            end: cell,
        };
    }

    pub fn tracks(&self) -> Range<usize> {
        return self.anchor.track.min(self.end.track)..self.anchor.track.max(self.end.track) + 1;
    }

    pub fn lines(&self) -> Range<usize> {
        return self.anchor.line.min(self.end.line)..self.anchor.line.max(self.end.line) + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_corner_can_come_first() {
        let mut range = BlockRange::at(PatternId(0), Cell { track: 2, line: 9 });
        range.end = Cell { track: 0, line: 4 };
        assert_eq!((range.tracks(), range.lines()), (0..3, 4..10));
    }
}
