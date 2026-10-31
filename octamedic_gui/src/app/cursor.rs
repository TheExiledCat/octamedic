// the note, then two digits each for the instrument, command and value
pub const TRACK_COLUMNS: usize = 7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditCursor {
    pub edit_mode: bool,
    pub line: usize,
    pub track: usize,
    pub column: usize,
}

impl EditCursor {
    pub fn move_lines(&mut self, delta: isize, lines: usize) {
        if lines > 0 {
            self.line = wrap(self.line, delta, lines);
        }
    }

    pub fn set_line(&mut self, line: usize, lines: usize) {
        self.line = line.min(lines.saturating_sub(1));
    }

    pub fn move_columns(&mut self, delta: isize, tracks: usize) {
        if tracks == 0 {
            return;
        }
        let position = self.track * TRACK_COLUMNS + self.column;
        let position = wrap(position, delta, tracks * TRACK_COLUMNS);
        self.track = position / TRACK_COLUMNS;
        self.column = position % TRACK_COLUMNS;
    }

    pub fn move_tracks(&mut self, delta: isize, tracks: usize) {
        if tracks > 0 {
            self.track = wrap(self.track, delta, tracks);
        }
    }

    pub fn move_tracks_clamped(&mut self, delta: isize, tracks: usize) {
        let last = tracks.saturating_sub(1) as isize;
        self.track = (self.track as isize + delta).clamp(0, last) as usize;
    }

    pub fn clamp(&mut self, lines: usize, tracks: usize) {
        self.line = self.line.min(lines.saturating_sub(1));
        self.track = self.track.min(tracks.saturating_sub(1));
    }
}

fn wrap(value: usize, delta: isize, count: usize) -> usize {
    return (value as isize + delta).rem_euclid(count as isize) as usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: usize, track: usize, column: usize) -> EditCursor {
        return EditCursor {
            line,
            track,
            column,
            edit_mode: false,
        };
    }

    #[test]
    fn lines_wrap() {
        let mut cursor = at(0, 0, 0);
        cursor.move_lines(-1, 64);
        assert_eq!(cursor.line, 63);
        cursor.move_lines(1, 64);
        assert_eq!(cursor.line, 0);
    }

    #[test]
    fn columns_wrap() {
        let mut cursor = at(0, 0, TRACK_COLUMNS - 1);
        cursor.move_columns(1, 4);
        assert_eq!(cursor, at(0, 1, 0));
        cursor = at(0, 0, 0);
        cursor.move_columns(-1, 4);
        assert_eq!(cursor, at(0, 3, TRACK_COLUMNS - 1));
    }

    #[test]
    fn tracks_keep_column() {
        let mut cursor = at(0, 3, 2);
        cursor.move_tracks(1, 4);
        assert_eq!(cursor, at(0, 0, 2));
        cursor.move_tracks_clamped(-8, 16);
        assert_eq!(cursor, at(0, 0, 2));
        cursor.move_tracks_clamped(20, 16);
        assert_eq!(cursor, at(0, 15, 2));
    }

    #[test]
    fn line_past_end() {
        let mut cursor = at(0, 0, 0);
        cursor.set_line(100, 64);
        assert_eq!(cursor.line, 63);
    }

    #[test]
    fn empty_block() {
        let mut cursor = at(3, 1, 2);
        cursor.move_lines(1, 0);
        cursor.move_columns(1, 0);
        cursor.move_tracks(1, 0);
        assert_eq!(cursor, at(3, 1, 2));
    }
}
