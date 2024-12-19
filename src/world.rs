#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;

use array2d::{Array2D /*, Error */};

#[derive(Clone, Debug)]
pub struct WorldGrid {
    cells: Array2D<GridCell>,
    // Should always be the same size as `cells`. When updating, we read from
    // `cells` and write to `scratch_cells`, then swap. Otherwise, it's not in
    // use, and `cells` should be updated directly.
    next_cells: Array2D<GridCell>,
}

impl WorldGrid {
    pub fn new_random(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        // result.randomize();
        result.cells[(height / 2, width / 2)] = GridCell::new(1.0);
        result
    }

    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: Array2D::filled_with(GridCell::default(), height, width),
            next_cells: Array2D::filled_with(GridCell::default(), height, width),
        }
    }

    pub fn randomize(&mut self) {
        let mut rng: randomize::PCG32 = generate_seed().into();
        for i in 0..self.cells.num_elements() {
            let cell = self.cells.get_mut_row_major(i).unwrap();
            *cell = GridCell::new(randomize::f32_closed(rng.next_u32()));
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
        self.init_next_cells();
        self.update_next_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn init_next_cells(&mut self) {
        for i in 0..self.cells.num_elements() {
            let cell = self.cells.get_row_major(i).unwrap();
            let next_cell = self.next_cells.get_mut_row_major(i).unwrap();
            *next_cell = *cell;
        }
    }

    fn update_next_cells(&mut self) {
        for row in 0..self.height() {
            for col in 0..self.width() {
                self.cells[(row, col)].update_next_cells(row, col, &mut self.next_cells);
            }
        }
    }

    // fn count_neighbors(&self, row: usize, col: usize) -> usize {
    //     let (col_left, col_right) = neighbor_indexes(col, self.width() - 1);
    //     let (row_above, row_below) = neighbor_indexes(row, self.height() - 1);
    //     self.cells[(row_above, col_left)].alive as usize
    //        + self.cells[(row_above, col)].alive as usize
    //        + self.cells[(row_above, col_right)].alive as usize
    //        + self.cells[(row, col_left)].alive as usize
    //        + self.cells[(row, col_right)].alive as usize
    //        + self.cells[(row_below, col_left)].alive as usize
    //        + self.cells[(row_below, col)].alive as usize
    //        + self.cells[(row_below, col_right)].alive as usize
    // }
}

/// Generate a pseudorandom seed for the game's PRNG.
fn generate_seed() -> (u64, u64) {
    use byteorder::{ByteOrder, NativeEndian};
    use getrandom::getrandom;

    let mut seed = [0_u8; 16];

    getrandom(&mut seed).expect("failed to getrandom");

    (
        NativeEndian::read_u64(&seed[0..8]),
        NativeEndian::read_u64(&seed[8..16]),
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridCell {
    pub substance: Substance,
}

impl GridCell {
    fn new(amount: f32) -> Self {
        Self {
            substance: Substance::new(amount),
        }
    }

    fn update_next_cells(&self, row: usize, col: usize, next_cells: &mut Array2D<GridCell>) {
        next_cells[(row, col)].substance.decay();
    }
}

// fn neighbor_indexes(cell_index: usize, max_index: usize) -> (usize, usize) {
//     if cell_index == 0 {
//         (max_index, 1)
//     } else if cell_index == max_index {
//         (max_index - 1, 0)
//     } else {
//         (cell_index - 1, cell_index + 1)
//     }
// }

#[derive(Clone, Copy, Debug, Default)]
pub struct Substance {
    pub color: [u8; 3],
    pub amount: f32,
}

impl Substance {
    fn new(amount: f32) -> Self {
        Self {
            color: [0xff, 0, 0],
            amount: amount.clamp(0.0, 1.0),
        }
    }

    fn decay(&mut self) {
        self.amount = (self.amount * 0.99).clamp(0.0, 1.0);
    }
}
