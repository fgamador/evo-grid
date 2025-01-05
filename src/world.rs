#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct World {
    cells: WorldGrid,
    next_cells: WorldGrid,
    sources: Vec<SubstanceSource>,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);

        result.sources.push(SubstanceSource::new(height / 4, width / 4, 3 * (width / 4),
                                                 Substance::new([0xff, 0, 0], 1.0)));
        // result.sources.push(SubstanceSource::new(height / 4, width / 4, 1 + width / 4,
        //                                          Substance::new([0xff, 0, 0], 1.0)));

        result.cells[(20 + height / 4, width / 3)].creature = Some(Creature::new([0, 0xff, 0]));

        // result.cells[(1 + height / 4, width / 2)].debug_selected = true;

        result
    }

    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: WorldGrid::new(width, height),
            next_cells: WorldGrid::new(width, height),
            sources: vec![],
        }
    }

    pub fn width(&self) -> usize {
        self.cells.width()
    }

    pub fn height(&self) -> usize {
        self.cells.height()
    }

    pub fn num_cells(&self) -> usize {
        self.cells.num_cells()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item=&GridCell> + Clone {
        self.cells.cells_iter()
    }

    pub fn update(&mut self) {
        self.next_cells.copy_from(&self.cells);
        self.update_next_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn update_next_cells(&mut self) {
        self.sources.iter().for_each(|source| source.update_cells(&mut self.next_cells));

        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_neighborhood(row, col);
            }
        }
    }

    fn update_neighborhood(&mut self, row: usize, col: usize) {
        let cell = self.cells[(row, col)];
        if cell.debug_selected {
            println!("{:?}", cell);
        }
        if !cell.is_empty() {
            let mut neighborhood = Neighborhood::new(self, row, col);
            cell.update_neighborhood(&mut neighborhood);
        }
    }
}

#[derive(Clone, Debug)]
struct WorldGrid {
    cells: Vec<GridCell>,
    width: usize,
    height: usize,
}

impl WorldGrid {
    fn new(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: vec![GridCell::default(); width * height],
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn num_cells(&self) -> usize {
        self.cells.len()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item=&GridCell> + Clone {
        self.cells.iter()
    }

    fn get(&self, row: usize, col: usize) -> Option<&GridCell> {
        self.get_index(row, col)
            .map(|index| &self.cells[index])
    }

    fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut GridCell> {
        self.get_index(row, col)
            .map(|index| &mut self.cells[index])
    }

    fn copy_from(&mut self, source: &Self) {
        self.cells.copy_from_slice(&source.cells);
    }

    fn get_index(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.height && col < self.width {
            Some(row * self.width + col)
        } else {
            None
        }
    }
}

impl Index<(usize, usize)> for WorldGrid {
    type Output = GridCell;

    fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
        self.get(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

impl IndexMut<(usize, usize)> for WorldGrid {
    fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut Self::Output {
        self.get_mut(row, column)
            .unwrap_or_else(|| panic!("Index_mut indices {}, {} out of bounds", row, column))
    }
}

#[derive(Debug)]
struct SubstanceSource {
    row: usize,
    min_col: usize,
    max_col: usize,
    substance: Substance,
}

impl SubstanceSource {
    fn new(row: usize, min_col: usize, max_col: usize, substance: Substance) -> Self {
        Self {
            row,
            min_col,
            max_col,
            substance,
        }
    }

    fn update_cells(&self, grid: &mut WorldGrid)
    {
        for col in self.min_col..self.max_col {
            let substance = grid[(self.row, col)].substance.get_or_insert_default();
            *substance = self.substance;
        }
    }
}

struct Neighborhood<'a> {
    cells: &'a WorldGrid,
    next_cells: &'a mut WorldGrid,
    rows: [usize; 3],
    cols: [usize; 3],
}

impl<'a> Neighborhood<'a> {
    fn new(grid: &'a mut World, center_row: usize, center_col: usize) -> Self {
        let (row_above, row_below) = Self::adjacent_indexes(center_row, grid.height());
        let (col_left, col_right) = Self::adjacent_indexes(center_col, grid.width());
        Self {
            cells: &grid.cells,
            next_cells: &mut grid.next_cells,
            rows: [row_above, center_row, row_below],
            cols: [col_left, center_col, col_right],
        }
    }

    fn for_center<F>(&mut self, f: F)
    where
        F: Fn(&GridCell, &mut GridCell),
    {
        self.for_cell(1, 1, &f);
    }

    fn for_neighbors<F>(&mut self, f: F)
    where
        F: Fn(&GridCell, &mut GridCell),
    {
        self.for_cell(0, 0, &f);
        self.for_cell(0, 1, &f);
        self.for_cell(0, 2, &f);

        self.for_cell(1, 0, &f);
        self.for_cell(1, 2, &f);

        self.for_cell(2, 0, &f);
        self.for_cell(2, 1, &f);
        self.for_cell(2, 2, &f);
    }

    fn for_cell<F>(&mut self, row: usize, col: usize, f: &F)
    where
        F: Fn(&GridCell, &mut GridCell),
    {
        let grid_index = (self.rows[row], self.cols[col]);
        f(&self.cells[grid_index], &mut self.next_cells[grid_index]);
    }

    fn adjacent_indexes(cell_index: usize, max: usize) -> (usize, usize) {
        (
            (cell_index as i64 - 1).rem_euclid(max as i64) as usize,
            (cell_index as i64 + 1).rem_euclid(max as i64) as usize,
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridCell {
    pub creature: Option<Creature>,
    pub substance: Option<Substance>,
    pub debug_selected: bool,
}

impl GridCell {
    fn is_empty(&self) -> bool {
        self.creature.is_none() && self.substance.is_none()
    }

    fn update_neighborhood(&self, neighborhood: &mut Neighborhood) {
        if let Some(creature) = self.creature {
            creature.update_neighborhood(neighborhood);
        }

        if let Some(substance) = self.substance {
            substance.update_neighborhood(neighborhood);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Creature {
    pub color: [u8; 3],
    pub age: u64,
}

impl Creature {
    fn new(color: [u8; 3]) -> Self {
        Self {
            color,
            age: 0,
        }
    }

    fn update_neighborhood(&self, neighborhood: &mut Neighborhood) {
        neighborhood.for_center(|_cell, next_cell| {
            let next_creature = next_cell.creature.as_mut().unwrap();
            if next_creature.age > 3 {
                next_cell.creature = None;
            } else {
                next_creature.age += 1;
            }
        });

        if self.age == 0 {
            neighborhood.for_cell(0, 2, &|_neighbor, next_neighbor| {
                if next_neighbor.creature.is_none() {
                    next_neighbor.creature = Some(Creature::new(self.color));
                }
            });
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

    fn update_neighborhood(&self, neighborhood: &mut Neighborhood) {
        const DONATE_FRACTION: f32 = 0.1;
        const DECAY_FRACTION: f32 = 0.01;
        const MIN_AMOUNT: f32 = 0.01;

        neighborhood.for_center(|_cell, next_cell| {
            if self.amount < MIN_AMOUNT {
                next_cell.substance = None;
            } else {
                let next_substance = next_cell.substance.as_mut().unwrap();
                next_substance.amount -= (DONATE_FRACTION + DECAY_FRACTION) * self.amount;
            }
        });

        if self.amount >= MIN_AMOUNT {
            neighborhood.for_neighbors(|_neighbor, next_neighbor| {
                let next_substance = next_neighbor.substance.get_or_insert(
                    Substance::new(self.color, 0.0));
                if next_substance.color == self.color {
                    next_substance.amount += (DONATE_FRACTION / 8.0) * self.amount;
                }
            });
        }
    }
}
