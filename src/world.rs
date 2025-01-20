#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;
use std::ops::{Index, IndexMut, Range};

use rand::prelude::*;

#[derive(Debug)]
pub struct World {
    cells: WorldGrid,
    next_cells: WorldGrid,
    sources: Vec<SubstanceSource>,
    rand: Random,
}

impl World {
    pub fn new(width: usize, height: usize, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_substances();
        result.add_creatures();
        // result.cells[(1 + height / 4, width / 2)].debug_selected = true;
        result
    }

    fn new_empty(width: usize, height: usize, rand: Random) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: WorldGrid::new(width, height),
            next_cells: WorldGrid::new(width, height),
            sources: vec![],
            rand,
        }
    }

    fn add_substances(&mut self) {
        self.add_substance_source_clusters(40, 5, 10);
    }

    fn add_substance_source_clusters(&mut self, count: usize, radius: usize, size: usize) {
        for _ in 0..count {
            let row = self.rand.next_usize(radius..(self.height() - radius));
            let col = self.rand.next_usize(radius..(self.width() - radius));
            self.add_substance_source_cluster(Loc::new(row, col), radius, size);
        }
    }

    fn add_substance_source_cluster(&mut self, center: Loc, radius: usize, size: usize) {
        let substance = Substance::new(self.random_color(), 1.0);
        for _ in 0..size {
            let loc = Loc::new(self.random_offset(center.row, radius),
                               self.random_offset(center.col, radius));
            self.sources.push(SubstanceSource::new(loc, substance));
        }
    }

    fn random_color(&mut self) -> [u8; 3] {
        let result = [0xff, self.rand.next_u8(0..0xff), self.rand.next_u8(0..0x80)];
        self.rand.shuffle_color_rgb(result)
    }

    fn random_offset(&mut self, index: usize, max_offset: usize) -> usize {
        let offset_range = -(max_offset as i32)..max_offset as i32;
        (index as i32 + self.rand.next_i32(offset_range)) as usize
    }

    fn add_creatures(&mut self) {
        let loc = Loc::new(20 + self.height() / 4, self.width() / 3);
        self.cells[loc].creature = Some(Creature::new([0, 0xff, 0]));
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
                self.update_neighborhood(Loc::new(row, col));
            }
        }
    }

    fn update_neighborhood(&mut self, loc: Loc) {
        let cell = self.cells[loc];
        if cell.debug_selected {
            println!("{:?}", cell);
        }
        if !cell.is_empty() {
            let mut neighborhood = Neighborhood::new(self, loc);
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

    fn get(&self, loc: Loc) -> Option<&GridCell> {
        self.get_index(loc)
            .map(|index| &self.cells[index])
    }

    fn get_mut(&mut self, loc: Loc) -> Option<&mut GridCell> {
        self.get_index(loc)
            .map(|index| &mut self.cells[index])
    }

    fn copy_from(&mut self, source: &Self) {
        self.cells.copy_from_slice(&source.cells);
    }

    fn get_index(&self, loc: Loc) -> Option<usize> {
        if loc.row < self.height && loc.col < self.width {
            Some(loc.row * self.width + loc.col)
        } else {
            None
        }
    }
}

impl Index<Loc> for WorldGrid {
    type Output = GridCell;

    fn index(&self, loc: Loc) -> &Self::Output {
        self.get(loc)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", loc.row, loc.col))
    }
}

impl IndexMut<Loc> for WorldGrid {
    fn index_mut(&mut self, loc: Loc) -> &mut Self::Output {
        self.get_mut(loc)
            .unwrap_or_else(|| panic!("Index_mut indices {}, {} out of bounds", loc.row, loc.col))
    }
}

#[derive(Clone, Copy, Debug)]
struct Loc {
    row: usize,
    col: usize,
}

impl Loc {
    fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SubstanceSource {
    loc: Loc,
    substance: Substance,
}

impl SubstanceSource {
    fn new(loc: Loc, substance: Substance) -> Self {
        Self {
            loc,
            substance,
        }
    }

    fn update_cells(&self, grid: &mut WorldGrid)
    {
        let substance = grid[self.loc].substance.get_or_insert_default();
        *substance = self.substance;
    }
}

struct Neighborhood<'a> {
    cells: &'a WorldGrid,
    next_cells: &'a mut WorldGrid,
    rows: [usize; 3],
    cols: [usize; 3],
}

impl<'a> Neighborhood<'a> {
    fn new(grid: &'a mut World, center: Loc) -> Self {
        let (row_above, row_below) = Self::adjacent_indexes(center.row, grid.height());
        let (col_left, col_right) = Self::adjacent_indexes(center.col, grid.width());
        Self {
            cells: &grid.cells,
            next_cells: &mut grid.next_cells,
            rows: [row_above, center.row, row_below],
            cols: [col_left, center.col, col_right],
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
        let grid_index = Loc::new(self.rows[row], self.cols[col]);
        f(&self.cells[grid_index], &mut self.next_cells[grid_index]);
    }

    fn adjacent_indexes(cell_index: usize, max: usize) -> (usize, usize) {
        (
            Self::modulo(cell_index as i64 - 1, max),
            Self::modulo(cell_index as i64 + 1, max),
        )
    }

    fn modulo(val: i64, max: usize) -> usize {
        val.rem_euclid(max as i64) as usize
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
    const DONATE_FRACTION: f32 = 0.1;
    const DECAY_FRACTION: f32 = 0.01;
    const MIN_AMOUNT: f32 = 0.01;

    fn new(color: [u8; 3], amount: f32) -> Self {
        Self {
            color,
            amount: amount.clamp(0.0, 1.0),
        }
    }

    fn update_neighborhood(&self, neighborhood: &mut Neighborhood) {
        neighborhood.for_center(|_cell, next_cell| {
            if self.amount < Self::MIN_AMOUNT {
                next_cell.substance = None;
            } else {
                let next_substance = next_cell.substance.as_mut().unwrap();
                next_substance.amount -= (Self::DONATE_FRACTION + Self::DECAY_FRACTION) * self.amount;
            }
        });

        if self.amount >= Self::MIN_AMOUNT {
            neighborhood.for_neighbors(|_neighbor, next_neighbor| {
                let next_neighbor_substance = next_neighbor.substance.get_or_insert(
                    Substance::new(self.color, 0.0));
                if next_neighbor_substance.color == self.color {
                    next_neighbor_substance.amount += (Self::DONATE_FRACTION / 8.0) * self.amount;
                }
            });
        }
    }
}

#[derive(Debug)]
pub struct Random {
    rng: ThreadRng,
}

impl Random {
    pub fn new() -> Self {
        Self {
            rng: thread_rng(),
        }
    }

    fn next_usize(&mut self, range: Range<usize>) -> usize {
        self.rng.gen_range(range)
    }

    fn next_u8(&mut self, range: Range<u8>) -> u8 {
        self.rng.gen_range(range)
    }

    fn next_i32(&mut self, range: Range<i32>) -> i32 {
        self.rng.gen_range(range)
    }

    fn shuffle_color_rgb(&mut self, mut color: [u8; 3]) -> [u8; 3] {
        color.shuffle(&mut self.rng);
        color
    }
}
