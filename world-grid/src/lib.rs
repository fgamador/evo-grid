#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::fmt::Debug;
use std::mem;
use std::ops::{Index, IndexMut, Range};

use rand::prelude::*;

pub trait World {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn num_cells(&self) -> usize;
    fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &impl GridCell> + Clone;
    fn update(&mut self);
}

#[derive(Clone, Debug)]
pub struct WorldGrid<C>
where
    C: Clone + GridCell,
{
    width: usize,
    height: usize,
    cells: WorldGridCells<C>,
    next_cells: WorldGridCells<C>,
}

impl<C> WorldGrid<C>
where
    C: Clone + Debug + GridCell,
{
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            width,
            height,
            cells: WorldGridCells::new(width, height),
            next_cells: WorldGridCells::new(width, height),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn num_cells(&self) -> usize {
        self.cells.num_cells()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &C> + Clone {
        self.cells.cells_iter()
    }

    pub fn update(&mut self) {
        self.next_cells.copy_from(&self.cells);
        self.update_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn update_cells(&mut self) {
        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_cell(Loc::new(row, col));
            }
        }
    }

    fn update_cell(&mut self, loc: Loc) {
        let cell = &self.cells[loc];
        if cell.debug_selected() {
            println!("{:?}", cell);
        }

        let neighborhood = Neighborhood::new(&self.cells, loc);
        let next_cell = &mut self.next_cells[loc];
        cell.update(&neighborhood, next_cell);
    }
}

#[derive(Clone, Debug)]
pub struct WorldGridCells<C>
where
    C: Clone + GridCell,
{
    cells: Vec<C>,
    width: usize,
    height: usize,
}

impl<C> WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: vec![C::default(); width * height],
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

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &C> + Clone {
        self.cells.iter()
    }

    fn get(&self, loc: Loc) -> Option<&C> {
        self.get_index(loc).map(|index| &self.cells[index])
    }

    fn get_mut(&mut self, loc: Loc) -> Option<&mut C> {
        self.get_index(loc).map(|index| &mut self.cells[index])
    }

    pub fn copy_from(&mut self, source: &Self) {
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

impl<C> Index<Loc> for WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    type Output = C;

    fn index(&self, loc: Loc) -> &Self::Output {
        self.get(loc)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", loc.row, loc.col))
    }
}

impl<C> IndexMut<Loc> for WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    fn index_mut(&mut self, loc: Loc) -> &mut Self::Output {
        self.get_mut(loc)
            .unwrap_or_else(|| panic!("Index_mut indices {}, {} out of bounds", loc.row, loc.col))
    }
}

pub trait GridCell
where
    Self: Copy + Default,
{
    fn debug_selected(&self) -> bool;
    fn color_rgba(&self) -> [u8; 4];
    fn update(&self, neighborhood: &Neighborhood<Self>, next_cell: &mut Self);
}

// From https://en.wikipedia.org/wiki/Alpha_compositing
pub fn alpha_blend(above: [u8; 4], below: [u8; 4]) -> [u8; 4] {
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

pub struct Neighborhood<'a, C>
where
    C: Clone + Copy + Default + GridCell,
{
    cells: &'a WorldGridCells<C>,
    rows: [usize; 3],
    cols: [usize; 3],
}

impl<'a, C> Neighborhood<'a, C>
where
    C: Clone + Copy + Default + GridCell,
{
    pub fn new(cells: &'a WorldGridCells<C>, center: Loc) -> Self {
        let (row_above, row_below) = Self::adjacent_indexes(center.row, cells.height());
        let (col_left, col_right) = Self::adjacent_indexes(center.col, cells.width());
        Self {
            cells,
            rows: [row_above, center.row, row_below],
            cols: [col_left, center.col, col_right],
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> &C {
        let grid_index = Loc::new(self.rows[row], self.cols[col]);
        &self.cells[grid_index]
    }

    pub fn for_neighbor_cells<F>(&self, mut f: F)
    where
        F: FnMut(&C),
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
        F: FnMut(&C),
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

#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub row: usize,
    pub col: usize,
}

impl Loc {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
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

    pub fn next_bool(&mut self, p: f64) -> bool {
        self.rng.random_bool(p)
    }

    pub fn next_usize(&mut self, range: Range<usize>) -> usize {
        self.rng.random_range(range)
    }

    pub fn next_u8(&mut self, range: Range<u8>) -> u8 {
        self.rng.random_range(range)
    }

    pub fn next_i32(&mut self, range: Range<i32>) -> i32 {
        self.rng.random_range(range)
    }

    pub fn shuffle_color_rgb(&mut self, mut color: [u8; 3]) -> [u8; 3] {
        color.shuffle(&mut self.rng);
        color
    }
}
