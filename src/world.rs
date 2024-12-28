#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;

use array2d::{Array2D /*, Error */};

#[derive(Clone, Debug)]
pub struct WorldGrid {
    cells: Array2D<GridCell>,
    // Should always be the same size as `cells`. When updating, we read from
    // `cells` and write to `next_cells`, then swap. Otherwise, it's not in
    // use, and `cells` should be updated directly.
    next_cells: Array2D<GridCell>,
}

impl WorldGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.init_cell_square(0, 0, 10, [0xff, 0x00, 0xff]);
        result.init_cell_square(height / 2, width / 2, 10, [0xff, 0, 0]);
        result.init_cell_square(height / 2, (width / 2) - 20, 10, [0, 0xff, 0]);
        result.init_cell_square(height / 2, (width / 2) + 20, 10, [0, 0, 0xff]);
        result
    }

    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: Array2D::filled_with(GridCell::default(), height, width),
            next_cells: Array2D::filled_with(GridCell::default(), height, width),
        }
    }

    fn init_cell_square(&mut self, row0: usize, col0: usize, side: usize, color: [u8; 3]) {
        for row in row0..=(row0 + side) {
            for col in col0..=(col0 + side) {
                self.cells[(row, col)] = GridCell::new(color, 1.0);
            }
        }
    }

    pub fn width(&self) -> usize {
        self.cells.num_columns()
    }

    pub fn height(&self) -> usize {
        self.cells.num_rows()
    }

    pub fn num_cells(&self) -> usize {
        self.cells.num_elements()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item=&GridCell> + Clone {
        self.cells.elements_row_major_iter()
    }

    pub fn update(&mut self) {
        self.copy_cells_into_next_cells();
        self.update_next_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn copy_cells_into_next_cells(&mut self) {
        for i in 0..self.cells.num_elements() {
            let cell = self.cells.get_row_major(i).unwrap();
            let next_cell = self.next_cells.get_mut_row_major(i).unwrap();
            *next_cell = *cell;
        }
    }

    fn update_next_cells(&mut self) {
        let mut deltas = NeighborhoodDeltas::new();
        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_neighborhood(row, col, &mut deltas);
            }
        }
    }

    fn update_neighborhood(&mut self, row: usize, col: usize, mut deltas: &mut NeighborhoodDeltas) {
        let cell = self.cells[(row, col)];
        if cell.substance.is_some() {
            deltas.clear();
            cell.calc_neighborhood_deltas(&mut deltas);
            self.apply_neighborhood_deltas(row, col, &deltas);
        }
    }

    fn apply_neighborhood_deltas(&mut self, row: usize, col: usize, deltas: &NeighborhoodDeltas) {
        let (row_above, row_below) = adjacent_indexes(row, self.next_cells.num_rows() - 1);
        let (col_left, col_right) = adjacent_indexes(col, self.next_cells.num_columns() - 1);

        self.next_cells[(row_above, col_left)].apply_delta(&deltas.deltas[(0, 0)]);
        self.next_cells[(row_above, col)].apply_delta(&deltas.deltas[(0, 1)]);
        self.next_cells[(row_above, col_right)].apply_delta(&deltas.deltas[(0, 2)]);
        self.next_cells[(row, col_left)].apply_delta(&deltas.deltas[(1, 0)]);
        self.next_cells[(row, col)].apply_delta(&deltas.deltas[(1, 1)]);
        self.next_cells[(row, col_right)].apply_delta(&deltas.deltas[(1, 2)]);
        self.next_cells[(row_below, col_left)].apply_delta(&deltas.deltas[(2, 0)]);
        self.next_cells[(row_below, col)].apply_delta(&deltas.deltas[(2, 1)]);
        self.next_cells[(row_below, col_right)].apply_delta(&deltas.deltas[(2, 2)]);
    }
}

fn adjacent_indexes(cell_index: usize, max_index: usize) -> (usize, usize) {
    ((cell_index as i64 - 1).rem_euclid(max_index as i64 + 1) as usize,
     (cell_index as i64 + 1).rem_euclid(max_index as i64 + 1) as usize)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridCell {
    pub substance: Option<Substance>,
}

impl GridCell {
    fn new(color: [u8; 3], amount: f32) -> Self {
        Self {
            substance: Some(Substance::new(color, amount)),
        }
    }

    fn calc_neighborhood_deltas(&self, deltas: &mut NeighborhoodDeltas) {
        if let Some(substance) = self.substance {
            substance.calc_neighborhood_deltas(deltas);
        }
    }

    fn apply_delta(&mut self, delta: &GridCellDelta) {
        // TODO how do we remove the substance via a delta?
        let substance = self.substance.get_or_insert_default();
        if substance.color == [0, 0, 0] || delta.substance.color == substance.color {
            substance.apply_delta(&delta.substance);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Substance {
    pub color: [u8; 3],
    pub amount: f32,
}

impl Substance {
    fn new(color: [u8; 3], amount: f32) -> Self {
        Self {
            color,
            amount: amount.clamp(0.0, 1.0),
        }
    }

    fn calc_neighborhood_deltas(&self, deltas: &mut NeighborhoodDeltas) {
        deltas.for_all(|cell_delta| cell_delta.substance.color = self.color);
        deltas.for_center(|cell_delta| cell_delta.substance.amount = -0.11 * self.amount);
        deltas.for_neighbors(|cell_delta| cell_delta.substance.amount = (0.1 / 8.0) * self.amount);
    }

    fn apply_delta(&mut self, delta: &SubstanceDelta) {
        self.color = delta.color;
        self.set_amount_clamped(self.amount + delta.amount);
    }

    fn set_amount_clamped(&mut self, val: f32) {
        self.amount = val.clamp(0.0, 1.0);
    }
}

struct NeighborhoodDeltas {
    pub deltas: Array2D<GridCellDelta>,
}

impl NeighborhoodDeltas {
    fn new() -> Self {
        Self {
            deltas: Array2D::filled_with(GridCellDelta::default(), 3, 3),
        }
    }

    fn clear(&mut self) {
        self.for_all(|cell| cell.clear());
    }

    fn for_all<F>(&mut self, f: F)
    where
        F: Fn(&mut GridCellDelta),
    {
        for row in 0..=2 {
            for col in 0..=2 {
                f(&mut self.deltas[(row, col)]);
            }
        }
    }

    fn for_center<F>(&mut self, f: F)
    where
        F: Fn(&mut GridCellDelta),
    {
        f(&mut self.deltas[(1, 1)]);
    }

    fn for_neighbors<F>(&mut self, f: F)
    where
        F: Fn(&mut GridCellDelta),
    {
        f(&mut self.deltas[(0, 0)]);
        f(&mut self.deltas[(0, 1)]);
        f(&mut self.deltas[(0, 2)]);
        f(&mut self.deltas[(1, 0)]);
        f(&mut self.deltas[(1, 2)]);
        f(&mut self.deltas[(2, 0)]);
        f(&mut self.deltas[(2, 1)]);
        f(&mut self.deltas[(2, 2)]);
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GridCellDelta {
    pub substance: SubstanceDelta,
}

impl GridCellDelta {
    fn clear(&mut self) {
        self.substance.clear();
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SubstanceDelta {
    pub color: [u8; 3],
    pub amount: f32,
}

impl SubstanceDelta {
    fn clear(&mut self) {
        self.color = [0, 0, 0];
        self.amount = 0.0;
    }
}
