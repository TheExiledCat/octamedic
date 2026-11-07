use std::{collections::VecDeque, mem, sync::Arc};

use octamedic_core::data::{
    pattern::{OctamedicPattern, OctamedicPatternLine, OctamedicPatternTrack, PatternId},
    song::OctamedicSong,
};

// 1 MiB, then the oldest steps go
const HISTORY_BYTES: usize = 1 << 20;

/// Undo and redo for the blocks. Each record compares the blocks with the last one and keeps
/// only the cells that differ. A step holds the other version of those cells, so undo and redo
/// are the same swap.
#[derive(Default)]
pub struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    bytes: usize,
    // what `position` is with nothing to undo, which moves on as the oldest steps are dropped
    bottom: u64,
    next_id: u64,
    // the blocks at the last record, sharing the song's Arcs. The engine holds them too, so an
    // edit copies a block on write whether this is here or not
    baseline: Vec<Arc<OctamedicPattern>>,
}

/// Where an undo or redo happened, for the cursor to go to.
#[derive(Debug, PartialEq, Eq)]
pub struct Jump {
    pub patterns: Vec<PatternId>,
    pub line: Option<usize>,
    pub track: Option<usize>,
}

// everything changed between two records, undone and redone together
struct Step {
    id: u64,
    changes: Vec<(PatternId, PatternChange)>,
}

enum PatternChange {
    // a highlight only needs its line, it's flipped back
    Cells {
        cells: Vec<CellChange>,
        highlights: Vec<u16>,
    },
    // for a change of size, or when most of the block changed and this is smaller. It keeps the
    // name the block has now when swapped in, as names aren't undone
    Whole(Arc<OctamedicPattern>),
}

struct CellChange {
    line: u16,
    track: u8,
    other: OctamedicPatternTrack,
}

impl History {
    /// Forgets every step and starts over from `patterns`.
    pub fn reset(&mut self, patterns: &[Arc<OctamedicPattern>]) {
        self.undo.clear();
        self.redo.clear();
        self.bytes = 0;
        self.bottom = self.new_id();
        self.baseline = patterns.to_vec();
    }

    /// Tells the blocks apart as they've been: it's the same again only after undoing or redoing
    /// back to the same step, so it can tell whether the blocks are back to how they were saved.
    pub fn position(&self) -> u64 {
        return self.undo.back().map_or(self.bottom, |step| step.id);
    }

    fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        return self.next_id;
    }

    /// Takes the blocks as they are now without making a step, for a change undo leaves alone.
    pub fn accept(&mut self, patterns: &[Arc<OctamedicPattern>]) {
        if patterns.len() == self.baseline.len() {
            self.baseline = patterns.to_vec();
        }
    }

    /// Makes a step of whatever changed since the last record, and drops the redo. A change in
    /// the number of blocks renumbers them, so it starts over instead.
    pub fn record(&mut self, patterns: &[Arc<OctamedicPattern>]) {
        if patterns.len() != self.baseline.len() {
            self.reset(patterns);
            return;
        }
        let mut changes = Vec::new();
        for (index, (baseline, current)) in self.baseline.iter_mut().zip(patterns).enumerate() {
            if Arc::ptr_eq(baseline, current) {
                continue;
            }
            let previous = mem::replace(baseline, current.clone());
            if let Some(change) = PatternChange::between(previous, current) {
                changes.push((PatternId(index as u8), change));
            }
        }
        if changes.is_empty() {
            return;
        }
        changes.shrink_to_fit();
        let step = Step {
            id: self.new_id(),
            changes,
        };
        self.bytes -= self.redo.drain(..).map(|step| step.bytes()).sum::<usize>();
        self.bytes += step.bytes();
        self.undo.push_back(step);
        while self.bytes > HISTORY_BYTES && self.undo.len() > 1 {
            let oldest = self.undo.pop_front().expect("there's more than one");
            self.bytes -= oldest.bytes();
            self.bottom = oldest.id;
        }
    }

    pub fn can_undo(&self) -> bool {
        return !self.undo.is_empty();
    }

    pub fn can_redo(&self) -> bool {
        return !self.redo.is_empty();
    }

    pub fn undo(&mut self, song: &mut OctamedicSong) -> Option<Jump> {
        let step = self.undo.pop_back()?;
        let (step, jump) = self.swap(step, song);
        self.redo.push(step);
        return Some(jump);
    }

    pub fn redo(&mut self, song: &mut OctamedicSong) -> Option<Jump> {
        let step = self.redo.pop()?;
        let (step, jump) = self.swap(step, song);
        self.undo.push_back(step);
        return Some(jump);
    }

    fn swap(&mut self, mut step: Step, song: &mut OctamedicSong) -> (Step, Jump) {
        self.bytes -= step.bytes();
        for (id, change) in &mut step.changes {
            change.swap(*id, song);
            if let Some(current) = song.get_shared_pattern(id) {
                self.baseline[id.0 as usize] = current;
            }
        }
        self.bytes += step.bytes();
        let (line, track) = step.changes[0].1.first_position();
        let jump = Jump {
            patterns: step.changes.iter().map(|(id, _)| *id).collect(),
            line,
            track,
        };
        return (step, jump);
    }
}

impl Step {
    fn bytes(&self) -> usize {
        let entries = self.changes.capacity() * size_of::<(PatternId, PatternChange)>();
        let changes: usize = self.changes.iter().map(|(_, change)| change.bytes()).sum();
        return size_of::<Self>() + entries + changes;
    }
}

impl PatternChange {
    fn between(previous: Arc<OctamedicPattern>, current: &OctamedicPattern) -> Option<Self> {
        let same_shape = previous.line_count() == current.line_count()
            && previous.track_count() == current.track_count();
        if !same_shape {
            return Some(Self::Whole(previous));
        }
        let mut cells = Vec::new();
        let mut highlights = Vec::new();
        for (line, (before, after)) in previous.lines.iter().zip(&current.lines).enumerate() {
            if before.highlighted != after.highlighted {
                highlights.push(line as u16);
            }
            for (track, (before, after)) in before.tracks.iter().zip(&after.tracks).enumerate() {
                if before != after {
                    cells.push(CellChange {
                        line: line as u16,
                        track: track as u8,
                        other: *before,
                    });
                }
            }
        }
        if cells.is_empty() && highlights.is_empty() {
            return None;
        }
        cells.shrink_to_fit();
        highlights.shrink_to_fit();
        let change = Self::Cells { cells, highlights };
        if change.bytes() >= pattern_bytes(&previous) {
            return Some(Self::Whole(previous));
        }
        return Some(change);
    }

    fn swap(&mut self, id: PatternId, song: &mut OctamedicSong) {
        match self {
            Self::Cells { cells, highlights } => {
                let Some(pattern) = song.get_pattern_mut(id) else {
                    return;
                };
                for cell in cells {
                    let line = &mut pattern.lines[cell.line as usize];
                    mem::swap(&mut line.tracks[cell.track as usize], &mut cell.other);
                }
                for &line in &*highlights {
                    let line = &mut pattern.lines[line as usize];
                    line.highlighted = !line.highlighted;
                }
            }
            Self::Whole(other) => {
                let Some(current) = song.replace_shared_pattern(id, other.clone()) else {
                    return;
                };
                if other.name != current.name {
                    let swapped_in = song.get_pattern_mut(id).expect("just put there");
                    swapped_in.name = current.name.clone();
                }
                *other = current;
            }
        }
    }

    fn first_position(&self) -> (Option<usize>, Option<usize>) {
        return match self {
            Self::Cells { cells, highlights } => match (cells.first(), highlights.first()) {
                (Some(cell), _) => (Some(cell.line as usize), Some(cell.track as usize)),
                (None, Some(&line)) => (Some(line as usize), None),
                (None, None) => (None, None),
            },
            Self::Whole(_) => (None, None),
        };
    }

    fn bytes(&self) -> usize {
        return match self {
            Self::Cells { cells, highlights } => {
                cells.capacity() * size_of::<CellChange>()
                    + highlights.capacity() * size_of::<u16>()
            }
            Self::Whole(pattern) => pattern_bytes(pattern),
        };
    }
}

fn pattern_bytes(pattern: &OctamedicPattern) -> usize {
    let lines = pattern.lines.capacity() * size_of::<OctamedicPatternLine>();
    let cells: usize = pattern
        .lines
        .iter()
        .map(|line| line.tracks.capacity() * size_of::<OctamedicPatternTrack>())
        .sum();
    return size_of::<OctamedicPattern>() + pattern.name.capacity() + lines + cells;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song() -> OctamedicSong {
        let mut song = OctamedicSong::new("test");
        song.get_pattern_mut(PatternId(0))
            .unwrap()
            .set_line_count(8);
        return song;
    }

    fn note(song: &OctamedicSong, line: usize, track: usize) -> u8 {
        return song.get_pattern(&PatternId(0)).unwrap().lines[line].tracks[track]
            .note
            .value;
    }

    fn set_note(song: &mut OctamedicSong, line: usize, track: usize, value: u8) {
        let pattern = song.get_pattern_mut(PatternId(0)).unwrap();
        pattern.lines[line].tracks[track].note.value = value;
    }

    #[test]
    fn a_cell_undoes_and_redoes() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 3, 1, 20);
        history.record(song.patterns());
        let jump = history.undo(&mut song).unwrap();
        assert_eq!(note(&song, 3, 1), 0);
        assert_eq!(
            jump,
            Jump {
                patterns: vec![PatternId(0)],
                line: Some(3),
                track: Some(1),
            }
        );
        // the undo itself isn't recorded as an edit
        history.record(song.patterns());
        assert!(!history.can_undo() && history.can_redo());
        assert!(history.redo(&mut song).is_some());
        assert_eq!(note(&song, 3, 1), 20);
        assert!(history.can_undo() && !history.can_redo());
        assert!(history.redo(&mut song).is_none());
    }

    #[test]
    fn only_the_changed_cells_are_kept() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 0, 0, 1);
        set_note(&mut song, 5, 2, 1);
        history.record(song.patterns());
        let PatternChange::Cells { cells, .. } = &history.undo[0].changes[0].1 else {
            panic!("expected cells");
        };
        assert_eq!(cells.len(), 2);
    }

    #[test]
    fn a_change_of_size_keeps_the_whole_block() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 7, 0, 9);
        history.record(song.patterns());
        song.get_pattern_mut(PatternId(0))
            .unwrap()
            .set_line_count(4);
        history.record(song.patterns());
        assert!(matches!(
            history.undo.back().unwrap().changes[0].1,
            PatternChange::Whole(_)
        ));
        history.undo(&mut song);
        assert_eq!(song.get_pattern(&PatternId(0)).unwrap().line_count(), 8);
        assert_eq!(note(&song, 7, 0), 9);
        history.redo(&mut song);
        assert_eq!(song.get_pattern(&PatternId(0)).unwrap().line_count(), 4);
    }

    #[test]
    fn names_arent_undone() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        song.get_pattern_mut(PatternId(0)).unwrap().name = "first".into();
        history.record(song.patterns());
        assert!(!history.can_undo());
        song.get_pattern_mut(PatternId(0))
            .unwrap()
            .set_line_count(4);
        history.record(song.patterns());
        song.get_pattern_mut(PatternId(0)).unwrap().name = "second".into();
        history.accept(song.patterns());
        history.undo(&mut song);
        let pattern = song.get_pattern(&PatternId(0)).unwrap();
        assert_eq!((pattern.line_count(), pattern.name.as_str()), (8, "second"));
    }

    #[test]
    fn position_comes_back_with_the_same_blocks() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        let saved = history.position();
        set_note(&mut song, 0, 0, 1);
        history.record(song.patterns());
        assert_ne!(history.position(), saved);
        history.undo(&mut song);
        assert_eq!(history.position(), saved);
        // a different edit from the same place isn't the saved one
        set_note(&mut song, 1, 0, 1);
        history.record(song.patterns());
        history.undo(&mut song);
        history.redo(&mut song);
        assert_ne!(history.position(), saved);
        history.reset(song.patterns());
        assert_ne!(history.position(), saved);
    }

    #[test]
    fn a_highlight_flips_back() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        song.get_pattern_mut(PatternId(0)).unwrap().lines[2].highlighted = true;
        history.record(song.patterns());
        history.undo(&mut song);
        assert!(!song.get_pattern(&PatternId(0)).unwrap().lines[2].highlighted);
    }

    #[test]
    fn a_new_edit_drops_the_redo() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 0, 0, 1);
        history.record(song.patterns());
        history.undo(&mut song);
        set_note(&mut song, 1, 0, 1);
        history.record(song.patterns());
        assert!(history.redo(&mut song).is_none());
    }

    #[test]
    fn nothing_changed_is_no_step() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 0, 0, 0);
        history.record(song.patterns());
        assert!(history.undo(&mut song).is_none());
    }

    #[test]
    fn adding_a_block_forgets_everything() {
        let mut song = song();
        let mut history = History::default();
        history.reset(song.patterns());
        set_note(&mut song, 0, 0, 1);
        history.record(song.patterns());
        song.insert_pattern(1, OctamedicPattern::new());
        history.record(song.patterns());
        assert!(history.undo(&mut song).is_none());
    }

    #[test]
    fn the_oldest_steps_go_past_the_limit() {
        let mut song = song();
        song.get_pattern_mut(PatternId(0))
            .unwrap()
            .set_line_count(256);
        let mut history = History::default();
        history.reset(song.patterns());
        let steps = 20_000;
        for step in 0..steps {
            set_note(&mut song, step % 256, 0, (step % 100) as u8 + 1);
            history.record(song.patterns());
        }
        assert!(history.bytes <= HISTORY_BYTES);
        assert!((1000..steps).contains(&history.undo.len()));
    }
}
