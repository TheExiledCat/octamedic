// nothing reaches into a grid's cells yet
#![allow(dead_code)]

use crate::{
    app::state::AppState,
    framework::{event::Event, painter::Painter, rect::Rect, widget::OctamedicWidget},
};

/// Widgets in rows of `columns`. When the area doesn't divide evenly, the spare pixels are spread
/// over the cells.
pub struct Grid<W> {
    columns: usize,
    rows: usize,
    cells: Vec<W>,
}

impl<W> Grid<W> {
    /// Cells past `columns * rows` are never shown.
    pub fn new(columns: usize, rows: usize, cells: Vec<W>) -> Self {
        return Self {
            columns,
            rows,
            cells,
        };
    }

    pub fn cells(&self) -> &[W] {
        return &self.cells;
    }

    pub fn cells_mut(&mut self) -> &mut [W] {
        return &mut self.cells;
    }

    pub fn cell_mut(&mut self, column: usize, row: usize) -> Option<&mut W> {
        if column >= self.columns || row >= self.rows {
            return None;
        }
        return self.cells.get_mut(row * self.columns + column);
    }

    pub fn cell_area(&self, area: Rect, index: usize) -> Rect {
        let (x, w) = span(area.x, area.w, index % self.columns, self.columns);
        let (y, h) = span(area.y, area.h, index / self.columns, self.rows);
        return Rect::new(x, y, w, h);
    }

    fn shown(&self) -> usize {
        return self.cells.len().min(self.columns * self.rows);
    }
}

impl<W: OctamedicWidget> OctamedicWidget for Grid<W> {
    fn update(&mut self, state: &mut AppState) {
        for cell in &mut self.cells {
            cell.update(state);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let shown = self.shown();
        return self.cells[..shown]
            .iter_mut()
            .any(|cell| cell.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for index in 0..self.shown() {
            let cell_area = self.cell_area(area, index);
            self.cells[index].render(state, cell_area, painter);
        }
    }
}

/// Start and length of cell `index` of `count`.
fn span(start: i32, length: i32, index: usize, count: usize) -> (i32, i32) {
    let edge = |i: usize| start + length * i as i32 / count as i32;
    let from = edge(index);
    return (from, edge(index + 1) - from);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(columns: usize, rows: usize) -> Grid<()> {
        return Grid::new(columns, rows, vec![(); columns * rows]);
    }

    #[test]
    fn cells_go_along_the_rows_first() {
        let grid = grid(2, 5);
        let area = Rect::new(10, 20, 116, 50);
        assert_eq!(grid.cell_area(area, 0), Rect::new(10, 20, 58, 10));
        assert_eq!(grid.cell_area(area, 1), Rect::new(68, 20, 58, 10));
        assert_eq!(grid.cell_area(area, 2), Rect::new(10, 30, 58, 10));
        assert_eq!(grid.cell_area(area, 9), Rect::new(68, 60, 58, 10));
    }

    #[test]
    fn the_cells_fill_an_area_that_doesnt_divide_evenly() {
        let grid = grid(8, 2);
        let area = Rect::new(0, 0, 54, 11);
        let widths: Vec<i32> = (0..8).map(|i| grid.cell_area(area, i).w).collect();
        assert_eq!(widths.iter().sum::<i32>(), 54);
        assert!(widths.iter().all(|&w| w == 6 || w == 7));
        let last = grid.cell_area(area, 15);
        assert_eq!(last.right(), 54);
        assert_eq!(last.bottom(), 11);
    }

    #[test]
    fn cells_outside_the_grid_are_none() {
        let mut grid = grid(2, 2);
        assert!(grid.cell_mut(1, 1).is_some());
        assert!(grid.cell_mut(2, 0).is_none());
        assert!(grid.cell_mut(0, 2).is_none());
    }
}
