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
            let loc = Loc::new(
                self.random_offset(center.row, radius),
                self.random_offset(center.col, radius),
            );
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

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &GridCell> + Clone {
        self.cells.cells_iter()
    }

    pub fn update(&mut self) {
        self.next_cells.copy_from(&self.cells);
        self.update_next_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn update_next_cells(&mut self) {
        self.sources
            .iter()
            .for_each(|source| source.update_cells(&mut self.next_cells));

        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_neighborhood(Loc::new(row, col));
                // self.update_next_cell(Loc::new(row, col));
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

    fn update_next_cell(&mut self, loc: Loc) {
        let cell = &self.cells[loc];
        if cell.debug_selected {
            println!("{:?}", cell);
        }

        let neighborhood = Neighborhood2::new(&self.cells, loc);
        let next_cell = &mut self.next_cells[loc];
        cell.update_next_cell(&neighborhood, next_cell);
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

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &GridCell> + Clone {
        self.cells.iter()
    }

    fn get(&self, loc: Loc) -> Option<&GridCell> {
        self.get_index(loc).map(|index| &self.cells[index])
    }

    fn get_mut(&mut self, loc: Loc) -> Option<&mut GridCell> {
        self.get_index(loc).map(|index| &mut self.cells[index])
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
        Self { row, col }
    }
}

#[derive(Clone, Copy, Debug)]
struct SubstanceSource {
    loc: Loc,
    substance: Substance,
}

impl SubstanceSource {
    fn new(loc: Loc, substance: Substance) -> Self {
        Self { loc, substance }
    }

    fn update_cells(&self, grid: &mut WorldGrid) {
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
    fn new(world: &'a mut World, center: Loc) -> Self {
        let (row_above, row_below) = Self::adjacent_indexes(center.row, world.height());
        let (col_left, col_right) = Self::adjacent_indexes(center.col, world.width());
        Self {
            cells: &world.cells,
            next_cells: &mut world.next_cells,
            rows: [row_above, center.row, row_below],
            cols: [col_left, center.col, col_right],
        }
    }

    fn for_center_cell<F>(&mut self, f: F)
    where
        F: Fn(&GridCell, &mut GridCell),
    {
        self.for_cell(1, 1, &f);
    }

    fn for_neighbor_cells<F>(&mut self, f: F)
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

struct Neighborhood2<'a> {
    cells: &'a WorldGrid,
    rows: [usize; 3],
    cols: [usize; 3],
}

impl<'a> Neighborhood2<'a> {
    fn new(cells: &'a WorldGrid, center: Loc) -> Self {
        let (row_above, row_below) = Self::adjacent_indexes(center.row, cells.height());
        let (col_left, col_right) = Self::adjacent_indexes(center.col, cells.width());
        Self {
            cells,
            rows: [row_above, center.row, row_below],
            cols: [col_left, center.col, col_right],
        }
    }

    fn center_cell(&self) -> &GridCell {
        self.cell(1, 1)
    }

    fn cell(&self, row: usize, col: usize) -> &GridCell {
        let grid_index = Loc::new(self.rows[row], self.cols[col]);
        &self.cells[grid_index]
    }

    fn for_neighbor_cells<F>(&self, mut f: F)
    where
        F: FnMut(&GridCell),
    {
        self.for_cell(0, 0, &mut f);
        self.for_cell(0, 1, &mut f);
        self.for_cell(0, 2, &mut f);

        self.for_cell(1, 0, &mut f);
        self.for_cell(1, 2, &mut f);

        self.for_cell(2, 0, &mut f);
        self.for_cell(2, 1, &mut f);
        self.for_cell(2, 2, &mut f);
    }

    fn for_cell<F>(&self, row: usize, col: usize, f: &mut F)
    where
        F: FnMut(&GridCell),
    {
        let grid_index = Loc::new(self.rows[row], self.cols[col]);
        f(&self.cells[grid_index]);
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

    fn update_next_cell(&self, neighborhood: &Neighborhood2, next_cell: &mut GridCell) {
        self.update_next_creature(neighborhood, next_cell);
        self.update_next_substance(neighborhood, next_cell);
    }

    fn update_next_creature(&self, neighborhood: &Neighborhood2, next_cell: &mut GridCell) {
        if let Some(creature) = self.creature {
            creature.update_next_cell(neighborhood, next_cell);
        } else {
            let sw_neighbor = neighborhood.cell(2, 0);
            if let Some(sw_creature) = sw_neighbor.creature {
                if sw_creature.age == 0 {
                    next_cell.creature = Some(Creature::new(sw_creature.color));
                }
            }
        }
    }

    fn update_next_substance(&self, neighborhood: &Neighborhood2, next_cell: &mut GridCell) {
        if let Some(substance) = self.substance {
            substance.update_next_cell(neighborhood, next_cell);
        }
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        alpha_blend(self.render_substance(), self.render_creature())
    }

    fn render_creature(&self) -> [u8; 4] {
        self.creature
            .map_or([0, 0, 0, 0], |creature| creature.color_rgba())
    }

    fn render_substance(&self) -> [u8; 4] {
        self.substance
            .map_or([0, 0, 0, 0], |substance| substance.color_rgba())
    }
}

// From https://en.wikipedia.org/wiki/Alpha_compositing
fn alpha_blend(above: [u8; 4], below: [u8; 4]) -> [u8; 4] {
    let above = color_as_fractions(above);
    let below = color_as_fractions(below);

    let above_alpha = above[3];
    let below_alpha = below[3];
    let result_alpha = above_alpha + below_alpha * (1.0 - above_alpha);

    let mut result: [f32; 4] = [0.0, 0.0, 0.0, result_alpha];
    for i in 0..=2 {
        result[i] =
            (above[i] * above_alpha + below[i] * below_alpha * (1.0 - above_alpha)) / result_alpha;
    }
    color_as_bytes(result)
}

fn color_as_fractions(color: [u8; 4]) -> [f32; 4] {
    let mut result: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
    for i in 0..=3 {
        result[i] = color[i] as f32 / 0xff as f32;
    }
    result
}

fn color_as_bytes(color: [f32; 4]) -> [u8; 4] {
    let mut result: [u8; 4] = [0, 0, 0, 0];
    for i in 0..=3 {
        result[i] = (color[i] * 0xff as f32) as u8;
    }
    result
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Creature {
    pub color: [u8; 3],
    pub age: u64,
}

impl Creature {
    fn new(color: [u8; 3]) -> Self {
        Self { color, age: 0 }
    }

    fn update_neighborhood(&self, neighborhood: &mut Neighborhood) {
        neighborhood.for_center_cell(|_cell, next_cell| {
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

    fn update_next_cell(&self, neighborhood: &Neighborhood2, next_cell: &mut GridCell) {
        if self.age > 3 {
            next_cell.creature = None;
        } else {
            let next_creature = next_cell.creature.as_mut().unwrap();
            next_creature.age += 1;
        }
    }

    fn color_rgba(&self) -> [u8; 4] {
        let color_rgb = self.color;
        [color_rgb[0], color_rgb[1], color_rgb[2], 0xff]
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
        neighborhood.for_center_cell(|_cell, next_cell| {
            if self.amount < Self::MIN_AMOUNT {
                next_cell.substance = None;
            } else {
                let next_substance = next_cell.substance.as_mut().unwrap();
                next_substance.amount -=
                    (Self::DONATE_FRACTION + Self::DECAY_FRACTION) * self.amount;
            }
        });

        if self.amount >= Self::MIN_AMOUNT {
            neighborhood.for_neighbor_cells(|_neighbor, next_neighbor| {
                let next_neighbor_substance = next_neighbor
                    .substance
                    .get_or_insert(Substance::new(self.color, 0.0));
                if next_neighbor_substance.color == self.color {
                    next_neighbor_substance.amount += (Self::DONATE_FRACTION / 8.0) * self.amount;
                }
            });
        }
    }

    fn update_next_cell(&self, neighborhood: &Neighborhood2, next_cell: &mut GridCell) {
        let next_substance = next_cell.substance.as_mut().unwrap();

        next_substance.amount += Self::sum_donations(neighborhood, self.color);

        if next_substance.amount < Self::MIN_AMOUNT {
            next_cell.substance = None;
        } else {
            next_substance.amount -= (Self::DONATE_FRACTION + Self::DECAY_FRACTION) * self.amount;
        }
    }

    fn sum_donations(neighborhood: &Neighborhood2, color: [u8; 3]) -> f32 {
        let mut donated: f32 = 0.0;
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(neighbor_substance) = neighbor.substance {
                if neighbor_substance.amount >= Self::MIN_AMOUNT
                    && neighbor_substance.color == color
                {
                    donated += (Self::DONATE_FRACTION / 8.0) * neighbor_substance.amount;
                }
            }
        });
        donated
    }

    fn color_rgba(&self) -> [u8; 4] {
        let color_rgb = self.color;
        let color_alpha = (self.amount * 0xff as f32) as u8; // .max(0x99);
        [color_rgb[0], color_rgb[1], color_rgb[2], color_alpha]
    }
}

#[derive(Debug)]
pub struct Random {
    rng: ThreadRng,
}

impl Random {
    pub fn new() -> Self {
        Self { rng: rand::rng() }
    }

    fn next_usize(&mut self, range: Range<usize>) -> usize {
        self.rng.random_range(range)
    }

    fn next_u8(&mut self, range: Range<u8>) -> u8 {
        self.rng.random_range(range)
    }

    fn next_i32(&mut self, range: Range<i32>) -> i32 {
        self.rng.random_range(range)
    }

    fn shuffle_color_rgb(&mut self, mut color: [u8; 3]) -> [u8; 3] {
        color.shuffle(&mut self.rng);
        color
    }
}
