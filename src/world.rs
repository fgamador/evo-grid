#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;
use std::ops::{Index, IndexMut};

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
                self.cells[(row, col)] = GridCell::new(None, Some(Substance {
                    color,
                    amount: 1.0,
                }));
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

    fn cell(&self, row: usize, column: usize) -> &GridCell {
        &self.cells[(row, column)]
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
        if !cell.is_empty() {
            deltas.clear();
            cell.calc_neighborhood_deltas(&mut deltas);
            self.apply_neighborhood_deltas(row, col, &deltas);
        }
    }

    fn apply_neighborhood_deltas(&mut self, row: usize, col: usize, deltas: &NeighborhoodDeltas) {
        let (row_above, row_below) = adjacent_indexes(row, self.next_cells.num_rows() - 1);
        let (col_left, col_right) = adjacent_indexes(col, self.next_cells.num_columns() - 1);

        self.next_cells[(row_above, col_left)].apply_delta(&deltas[(0, 0)]);
        self.next_cells[(row_above, col)].apply_delta(&deltas[(0, 1)]);
        self.next_cells[(row_above, col_right)].apply_delta(&deltas[(0, 2)]);
        self.next_cells[(row, col_left)].apply_delta(&deltas[(1, 0)]);
        self.next_cells[(row, col)].apply_delta(&deltas[(1, 1)]);
        self.next_cells[(row, col_right)].apply_delta(&deltas[(1, 2)]);
        self.next_cells[(row_below, col_left)].apply_delta(&deltas[(2, 0)]);
        self.next_cells[(row_below, col)].apply_delta(&deltas[(2, 1)]);
        self.next_cells[(row_below, col_right)].apply_delta(&deltas[(2, 2)]);
    }
}

fn adjacent_indexes(cell_index: usize, max_index: usize) -> (usize, usize) {
    (
        (cell_index as i64 - 1).rem_euclid(max_index as i64 + 1) as usize,
        (cell_index as i64 + 1).rem_euclid(max_index as i64 + 1) as usize,
    )
}

struct Neighborhood<'a> {
    grid: &'a WorldGrid,
    array: Array3By3<(usize, usize)>,
}

impl<'a> Neighborhood<'a> {
    fn new(grid: &'a WorldGrid) -> Self {
        Self {
            grid,
            array: Array3By3::<(usize, usize)>::new(),
        }
    }

    pub fn get(&self, row: usize, column: usize) -> Option<&'a GridCell> {
        let (grid_row, grid_column) = self.array.get(row, column)?;
        Some(self.grid.cell(*grid_row, *grid_column))
    }

    fn for_all<F>(&self, f: F)
    where
        F: Fn(&(usize, usize), bool),
    {
        self.array.for_all(f);
    }
}

impl<'a> Index<(usize, usize)> for Neighborhood<'a> {
    type Output = GridCell;

    fn index(&self, (row, column): (usize, usize)) -> &'a Self::Output {
        self.get(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridCell {
    pub creature: Option<Creature>,
    pub substance: Option<Substance>,
}

impl GridCell {
    fn new(creature: Option<Creature>, substance: Option<Substance>) -> Self {
        Self {
            creature,
            substance,
        }
    }

    fn is_empty(&self) -> bool {
        self.creature.is_none() && self.substance.is_none()
    }

    fn calc_neighborhood_deltas(&self, deltas: &mut NeighborhoodDeltas) {
        if let Some(creature) = self.creature {
            creature.calc_neighborhood_deltas(deltas);
        }

        if let Some(substance) = self.substance {
            substance.calc_neighborhood_deltas(deltas);
        }
    }

    fn apply_delta(&mut self, delta: &GridCellDelta) {
        if let Some(creature_delta) = delta.creature {
            // TODO
        }

        if let Some(substance_delta) = delta.substance {
            if self.substance.is_none() || substance_delta.color == self.substance.unwrap().color {
                let substance = self.substance.get_or_insert_default();
                substance.apply_delta(&substance_delta);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Creature {
    pub color: [u8; 3],
}

impl Creature {
    fn new(color: [u8; 3]) -> Self {
        Self { color }
    }

    fn calc_neighborhood_deltas(&self, deltas: &mut NeighborhoodDeltas) {
        // TODO
        // deltas.for_all(|cell_delta| cell_delta.creature.color = self.color);
    }

    fn apply_delta(&mut self, delta: &CreatureDelta) {
        self.color = delta.color;
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
        deltas.for_all_mut(|cell_delta, is_center| {
            cell_delta.substance = Some(SubstanceDelta {
                color: self.color,
                amount: if is_center {
                    -0.11 * self.amount
                } else {
                    (0.1 / 8.0) * self.amount
                },
            });
        });
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
    array: Array3By3<GridCellDelta>,
}

impl NeighborhoodDeltas {
    fn new() -> Self {
        Self {
            array: Array3By3::<GridCellDelta>::new(),
        }
    }

    fn clear(&mut self) {
        self.array.for_all_mut(|cell, _| cell.clear());
    }

    pub fn get(&self, row: usize, column: usize) -> Option<&GridCellDelta> {
        self.array.get(row, column)
    }

    pub fn get_mut(&mut self, row: usize, column: usize) -> Option<&mut GridCellDelta> {
        self.array.get_mut(row, column)
    }

    fn for_all_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut GridCellDelta, bool),
    {
        self.array.for_all_mut(f);
    }
}

impl Index<(usize, usize)> for NeighborhoodDeltas {
    type Output = GridCellDelta;

    fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
        self.get(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

impl IndexMut<(usize, usize)> for NeighborhoodDeltas {
    fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut Self::Output {
        self.get_mut(row, column)
            .unwrap_or_else(|| panic!("IndexMut indices {}, {} out of bounds", row, column))
    }
}

struct Array3By3<T: Copy + Default> {
    array: [T; 9],
}

impl<T: Copy + Default> Array3By3<T> {
    fn new() -> Self {
        Self {
            array: [T::default(); 9],
        }
    }

    pub fn get(&self, row: usize, column: usize) -> Option<&T> {
        Self::get_index(row, column).map(|index| &self.array[index])
    }

    pub fn get_mut(&mut self, row: usize, column: usize) -> Option<&mut T> {
        Self::get_index(row, column)
            .map(move |index| &mut self.array[index])
    }

    fn for_all<F>(&self, f: F)
    where
        F: Fn(&T, bool),
    {
        for index in 0..9 {
            f(&self.array[index], index == 4);
        }
    }

    fn for_all_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut T, bool),
    {
        for index in 0..9 {
            f(&mut self.array[index], index == 4);
        }
    }

    fn get_index(row: usize, column: usize) -> Option<usize> {
        if row < 3 && column < 3 {
            Some(row * 3 + column)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GridCellDelta {
    pub creature: Option<CreatureDelta>,
    pub substance: Option<SubstanceDelta>,
}

impl GridCellDelta {
    fn clear(&mut self) {
        self.creature = None;
        self.substance = None;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CreatureDelta {
    pub color: [u8; 3],
}

#[derive(Clone, Copy, Debug, Default)]
struct SubstanceDelta {
    pub color: [u8; 3],
    pub amount: f32,
}
