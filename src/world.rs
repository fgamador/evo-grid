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

    fn get_cell(&self, row: usize, col: usize) -> Option<&GridCell> {
        self.cells.get(row, col)
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
        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_neighborhood(row, col);
            }
        }
    }

    fn update_neighborhood(&mut self, row: usize, col: usize) {
        let cell = self.cells[(row, col)];
        if !cell.is_empty() {
            let neighborhood = Neighborhood::new(self, row, col);
            let deltas = cell.calc_neighborhood_deltas(&neighborhood);
            self.apply_neighborhood_deltas(row, col, &deltas);
        }
    }

    fn apply_neighborhood_deltas(&mut self, row: usize, col: usize, deltas: &NeighborhoodDeltas) {
        let (row_above, row_below) = adjacent_indexes(row, self.next_cells.num_rows());
        let (col_left, col_right) = adjacent_indexes(col, self.next_cells.num_columns());

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

impl Index<(usize, usize)> for WorldGrid {
    type Output = GridCell;

    fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
        self.get_cell(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

struct Neighborhood<'a> {
    array: [&'a GridCell; 9],
}

impl<'a> Neighborhood<'a> {
    fn new(grid: &'a WorldGrid, center_row: usize, center_col: usize) -> Self {
        let (row_above, row_below) = adjacent_indexes(center_row, grid.height());
        let (col_left, col_right) = adjacent_indexes(center_col, grid.width());
        Self {
            array: [
                &grid[(row_above, col_left)], &grid[(row_above, center_col)], &grid[(row_above, col_right)],
                &grid[(center_row, col_left)], &grid[(center_row, center_col)], &grid[(center_row, col_right)],
                &grid[(row_below, col_left)], &grid[(row_below, center_col)], &grid[(row_below, col_right)],
            ],
        }
    }

    fn get(&self, row: usize, column: usize) -> Option<&'a GridCell> {
        Some(self.array[Self::get_index(row, column)?])
    }

    fn get_index(row: usize, column: usize) -> Option<usize> {
        if row < 3 && column < 3 {
            Some(row * 3 + column)
        } else {
            None
        }
    }

    fn for_all_neighbors<F>(&self, deltas: &mut NeighborhoodDeltas, f: F)
    where
        F: Fn(&GridCell, &mut GridCellDelta),
    {
        for index in 0..=3 {
            f(self.array[index], &mut deltas.array[index]);
        }
        for index in 5..=8 {
            f(self.array[index], &mut deltas.array[index]);
        }
    }

    fn for_center<F>(&self, deltas: &mut NeighborhoodDeltas, f: F)
    where
        F: Fn(&GridCell, &mut GridCellDelta),
    {
        f(self.array[4], &mut deltas.array[4]);
    }
}

impl<'a> Index<(usize, usize)> for Neighborhood<'a> {
    type Output = GridCell;

    fn index(&self, (row, column): (usize, usize)) -> &'a Self::Output {
        self.get(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

fn adjacent_indexes(cell_index: usize, max: usize) -> (usize, usize) {
    (
        (cell_index as i64 - 1).rem_euclid(max as i64) as usize,
        (cell_index as i64 + 1).rem_euclid(max as i64) as usize,
    )
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

    fn calc_neighborhood_deltas(&self, neighborhood: &Neighborhood) -> NeighborhoodDeltas {
        let mut deltas = NeighborhoodDeltas::new();

        if let Some(creature) = self.creature {
            creature.calc_neighborhood_deltas(neighborhood, &mut deltas);
        }

        if let Some(substance) = self.substance {
            substance.calc_neighborhood_deltas(neighborhood, &mut deltas);
        }

        deltas
    }

    fn apply_delta(&mut self, delta: &GridCellDelta) {
        if let Some(creature_delta) = delta.creature {
            self.creature.get_or_insert_default().apply_delta(&creature_delta);
        }

        if let Some(substance_delta) = delta.substance {
            self.substance.get_or_insert_default().apply_delta(&substance_delta);
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

    fn calc_neighborhood_deltas(&self, _neighborhood: &Neighborhood, _deltas: &mut NeighborhoodDeltas) {
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

    fn calc_neighborhood_deltas(&self, neighborhood: &Neighborhood, deltas: &mut NeighborhoodDeltas) {
        neighborhood.for_all_neighbors(deltas, |neighbor, neighbor_delta| {
            if neighbor.substance.is_none() || neighbor.substance.unwrap().color == self.color {
                neighbor_delta.substance = Some(SubstanceDelta {
                    color: self.color,
                    amount: (0.1 / 8.0) * self.amount,
                });
            }
        });

        neighborhood.for_center(deltas, |_cell, cell_delta| {
            cell_delta.substance = Some(SubstanceDelta {
                color: self.color,
                amount: -0.11 * self.amount,
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
    array: [GridCellDelta; 9],
}

impl NeighborhoodDeltas {
    fn new() -> Self {
        Self {
            array: [GridCellDelta::default(); 9],
        }
    }

    fn get(&self, row: usize, column: usize) -> Option<&GridCellDelta> {
        Some(&self.array[Self::get_index(row, column)?])
    }

    fn get_mut(&mut self, row: usize, column: usize) -> Option<&mut GridCellDelta> {
        Some(&mut self.array[Self::get_index(row, column)?])
    }

    fn get_index(row: usize, column: usize) -> Option<usize> {
        if row < 3 && column < 3 {
            Some(row * 3 + column)
        } else {
            None
        }
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

#[derive(Clone, Copy, Debug, Default)]
struct GridCellDelta {
    pub creature: Option<CreatureDelta>,
    pub substance: Option<SubstanceDelta>,
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
