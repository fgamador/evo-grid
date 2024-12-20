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
    pub fn new(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.cells[(height / 2, width / 2)] = GridCell::new([0xff, 0, 0], 1.0);
        result
    }

    pub fn new_random(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.randomize();
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
            *cell = GridCell::new([0xff, 0, 0], randomize::f32_closed(rng.next_u32()));
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
    pub substance: Option<Substance>,
}

impl GridCell {
    fn new(color: [u8; 3], amount: f32) -> Self {
        Self {
            substance: Some(Substance::new(color, amount)),
        }
    }

    fn update_next_cells(&self, row: usize, col: usize, next_cells: &mut Array2D<GridCell>) {
        if let Some(substance) = self.substance {
            let mut deltas = NeighborhoodDeltas::new();

            substance.calc_deltas(&mut deltas);

            let (row_above, row_below) = neighbor_indexes(row, next_cells.num_rows() - 1);
            let (col_left, col_right) = neighbor_indexes(col, next_cells.num_columns() - 1);

            next_cells[(row_above, col_left)].apply_delta(&deltas.deltas[(0, 0)]);
            next_cells[(row_above, col)].apply_delta(&deltas.deltas[(0, 1)]);
            next_cells[(row_above, col_right)].apply_delta(&deltas.deltas[(0, 2)]);
            next_cells[(row, col_left)].apply_delta(&deltas.deltas[(1, 0)]);
            next_cells[(row, col)].apply_delta(&deltas.deltas[(1, 1)]);
            next_cells[(row, col_right)].apply_delta(&deltas.deltas[(1, 2)]);
            next_cells[(row_below, col_left)].apply_delta(&deltas.deltas[(2, 0)]);
            next_cells[(row_below, col)].apply_delta(&deltas.deltas[(2, 1)]);
            next_cells[(row_below, col_right)].apply_delta(&deltas.deltas[(2, 2)]);
        }
    }

    fn apply_delta(&mut self, delta: &GridCellDelta) {
        self.substance.get_or_insert_default().apply_delta(&delta.substance);
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

    fn calc_deltas(&self, deltas: &mut NeighborhoodDeltas) {
        deltas.for_all(|cell_delta| cell_delta.substance.color = self.color);
        deltas.for_center(|cell_delta| cell_delta.substance.amount = -0.11 * self.amount);
        deltas.for_neighbors(|cell_delta| cell_delta.substance.amount = (0.1 / 8.0) * self.amount);
    }

    fn apply_delta(&mut self, delta: &SubstanceDelta) {
        self.color = delta.color;
        self.set_amount_clamped(self.amount + delta.amount);
    }

    // fn decay(&mut self) {
    //     self.set_amount(self.amount * 0.99);
    // }
    //
    // fn diffuse_out(&mut self) -> f32 {
    //     let delta = self.amount * 0.2;
    //     self.set_amount(self.amount - delta);
    //     delta
    // }
    //
    // fn diffuse_in(&mut self, delta: f32) {
    //     // TODO do better
    //     self.color = [0xff, 0, 0];
    //     self.set_amount(self.amount + delta);
    // }

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

#[derive(Clone, Copy, Debug, Default)]
struct SubstanceDelta {
    pub color: [u8; 3],
    pub amount: f32,
}

// impl SubstanceDelta {
//     fn new(substance: &Substance, amount: f32) -> Self {
//         Self {
//             color: substance.color,
//             amount: amount.clamp(-1.0, 1.0),
//         }
//     }
// }

fn neighbor_indexes(cell_index: usize, max_index: usize) -> (usize, usize) {
    if cell_index == 0 {
        (max_index, 1)
    } else if cell_index == max_index {
        (max_index - 1, 0)
    } else {
        (cell_index - 1, cell_index + 1)
    }
}
