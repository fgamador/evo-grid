#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use rand::distr::uniform::{SampleRange, SampleUniform};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rayon::prelude::*;
use std::fmt::Debug;
use std::mem;
use std::ops::{Index, IndexMut, Range};
use std::slice::ChunksExactMut;

pub trait World {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn num_cells(&self) -> usize;
    fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &impl GridCell> + Clone;
    fn update(&mut self);
    fn debug_print(&self, _row: u32, _col: u32) {}
}

#[derive(Clone, Debug)]
pub struct WorldGrid<C>
where
    C: Clone + GridCell,
{
    width: u32,
    height: u32,
    pub cells: WorldGridCells<C>,
    pub next_cells: WorldGridCells<C>,
}

impl<C> WorldGrid<C>
where
    C: Clone + Debug + GridCell,
{
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            width,
            height,
            cells: WorldGridCells::new(width, height),
            next_cells: WorldGridCells::new(width, height),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn num_cells(&self) -> usize {
        self.cells.num_cells()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &C> + Clone {
        self.cells.cells_iter()
    }

    pub fn update<F>(&mut self, rand: &mut Option<Random>, mut other_update: F)
    where
        F: FnMut(&mut Self),
    {
        self.next_cells.copy_from(&self.cells);
        other_update(self);
        self.update_cells(rand);
        mem::swap(&mut self.next_cells, &mut self.cells);
    }

    fn update_cells(&mut self, rand: &mut Option<Random>) {
        self.next_cells
            .par_rows_mut()
            .zip(Random::multi_fork_option(rand, self.width).par_iter_mut())
            .enumerate()
            .for_each(|(row, (row_next_cells, row_rand))| {
                Self::update_row(
                    row as u32,
                    &self.cells,
                    row_next_cells,
                    self.width,
                    row_rand,
                );
            });
    }

    fn update_row(
        row: u32,
        cells: &WorldGridCells<C>,
        next_cells_row: &mut [C],
        width: u32,
        rand: &mut Option<Random>,
    ) {
        for col in 0..width {
            Self::update_cell(Loc::new(row, col), cells, next_cells_row, rand);
        }
    }

    fn update_cell(
        loc: Loc,
        cells: &WorldGridCells<C>,
        next_cells_row: &mut [C],
        rand: &mut Option<Random>,
    ) {
        let cell = &cells[loc];
        let neighborhood = Neighborhood::new(cells, loc);
        let next_cell = &mut next_cells_row[loc.col as usize];
        cell.update(&neighborhood, next_cell, rand);
    }
}

#[derive(Clone, Debug)]
pub struct WorldGridCells<C>
where
    C: Clone + GridCell,
{
    cells: Vec<C>,
    width: u32,
    height: u32,
}

impl<C> WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            cells: vec![C::default(); width as usize * height as usize],
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn num_cells(&self) -> usize {
        self.cells.len()
    }

    pub fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &C> + Clone {
        self.cells.iter()
    }

    pub fn rows_mut(&mut self) -> ChunksExactMut<'_, C> {
        self.cells.chunks_exact_mut(self.width as usize)
    }

    pub fn par_rows_mut(&mut self) -> rayon::slice::ChunksExactMut<'_, C> {
        self.cells.par_chunks_exact_mut(self.width as usize)
    }

    fn cell(&self, loc: Loc) -> Option<&C> {
        loc.grid_index(self.width, self.height)
            .map(|index| &self.cells[index])
    }

    fn cell_mut(&mut self, loc: Loc) -> Option<&mut C> {
        loc.grid_index(self.width, self.height)
            .map(|index| &mut self.cells[index])
    }

    pub fn copy_from(&mut self, source: &Self) {
        self.cells.copy_from_slice(&source.cells);
    }
}

impl<C> Index<Loc> for WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    type Output = C;

    fn index(&self, loc: Loc) -> &Self::Output {
        self.cell(loc)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", loc.row, loc.col))
    }
}

impl<C> IndexMut<Loc> for WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    fn index_mut(&mut self, loc: Loc) -> &mut Self::Output {
        self.cell_mut(loc)
            .unwrap_or_else(|| panic!("Index_mut indices {}, {} out of bounds", loc.row, loc.col))
    }
}

pub trait GridCell
where
    Self: Copy + Default + Send + Sync,
{

    fn color_rgba(&self) -> [u8; 4];
    fn update(
        &self,
        neighborhood: &Neighborhood<Self>,
        next_cell: &mut Self,
        rand: &mut Option<Random>,
    );
}

// From https://en.wikipedia.org/wiki/Alpha_compositing
pub fn alpha_blend(above: [u8; 4], below: [u8; 4]) -> [u8; 4] {
    if above[3] == 0xff {
        return above;
    }
    if above[3] == 0x00 {
        return below;
    }

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

// alpha_blend with below_alpha set to 1.0
pub fn alpha_blend_with_background(above: [u8; 4], below: [u8; 4]) -> [u8; 4] {
    if above[3] == 0xff {
        return above;
    }
    if above[3] == 0x00 {
        return below;
    }

    let above = color_as_fractions(above);
    let below = color_as_fractions(below);

    let above_alpha = above[3];

    let mut result: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    for i in 0..=2 {
        result[i] = above[i] * above_alpha + below[i] * (1.0 - above_alpha);
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
    center: Loc,
    cells: &'a WorldGridCells<C>,
}

impl<'a, C> Neighborhood<'a, C>
where
    C: Clone + Copy + Default + GridCell,
{
    pub fn new(cells: &'a WorldGridCells<C>, center: Loc) -> Self {
        Self { center, cells }
    }

    pub fn for_neighbor_cells<F>(&self, mut f: F)
    where
        F: FnMut(&C),
    {
        for row in Self::index_range(self.center.row, self.cells.height) {
            for col in Self::index_range(self.center.col, self.cells.width) {
                let loc = Loc::new(row, col);
                if loc != self.center {
                    f(&self.cells[loc]);
                }
            }
        }
    }

    fn index_range(center: u32, max: u32) -> Range<u32> {
        center.saturating_sub(1)..(center + 2).min(max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loc {
    pub row: u32,
    pub col: u32,
}

impl Loc {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    pub fn grid_index(&self, width: u32, height: u32) -> Option<usize> {
        if self.row < height && self.col < width {
            Some((self.row * width + self.col) as usize)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Random {
    rng: SmallRng,
}

impl Random {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_rng(&mut rand::rng()),
        }
    }

    pub fn fork(&mut self) -> Self {
        Self {
            rng: SmallRng::from_rng(&mut self.rng),
        }
    }

    pub fn next_bool(&mut self, p: f64) -> bool {
        self.rng.random_bool(p)
    }

    pub fn next_in_range<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform,
        R: SampleRange<T>,
    {
        self.rng.random_range(range)
    }

    pub fn shuffle_color_rgb(&mut self, mut color: [u8; 3]) -> [u8; 3] {
        color.shuffle(&mut self.rng);
        color
    }

    pub fn multi_fork_option(rand: &mut Option<Random>, count: u32) -> Vec<Option<Random>> {
        (0..count)
            .map(|_| rand.as_mut().map(|rand| rand.fork()))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BitSet8Gene {
    pub bits: u8,
}

impl BitSet8Gene {
    pub fn new(bits: u8) -> Self {
        Self { bits }
    }

    pub fn empty() -> Self {
        Self::new(0)
    }

    pub fn is_bit_set(&self, index: usize) -> bool {
        self.bits & (1 << index) != 0
    }

    pub fn set_bit(&mut self, index: usize) {
        self.bits |= 1 << index;
    }

    pub fn flip_bit(&mut self, index: usize) {
        self.bits ^= 1 << index;
    }

    pub fn count_set_bits(&self) -> usize {
        let mut result = 0;
        for i in 0..8 {
            if self.is_bit_set(i) {
                result += 1;
            }
        }
        result
    }

    pub fn merge(genes: &ArrayVec<Self, 8>, rand: &mut Option<Random>, mutation_odds: f64) -> Self {
        let mut bit_counts = BitCountsMap::new();
        for bit_set in genes {
            bit_counts.increment(&bit_set);
        }
        bit_counts.as_bit_set(rand, mutation_odds)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BitCountsMap {
    ones: [u32; 8],
    zeros: [u32; 8],
}

impl BitCountsMap {
    fn new() -> Self {
        Self {
            ones: [0; 8],
            zeros: [0; 8],
        }
    }

    fn increment(&mut self, bits: &BitSet8Gene) {
        for i in 0..8 {
            if bits.is_bit_set(i) {
                self.ones[i] += 1;
            } else {
                self.zeros[i] += 1;
            }
        }
    }

    fn as_bit_set(&self, rand: &mut Option<Random>, mutation_odds: f64) -> BitSet8Gene {
        let mut result = BitSet8Gene::empty();
        for i in 0..8 {
            if Self::merge_counts(self.ones[i], self.zeros[i], rand) {
                result.set_bit(i);
            }
            if let Some(rand) = rand
                && rand.next_bool(mutation_odds)
            {
                result.flip_bit(i);
            }
        }
        result
    }

    fn merge_counts(num_ones: u32, num_zeros: u32, rand: &mut Option<Random>) -> bool {
        if num_ones == 0 {
            false
        } else if num_zeros == 0 {
            true
        } else {
            let odds = num_ones as f64 / (num_ones + num_zeros) as f64;
            rand.as_mut().unwrap().next_bool(odds)
        }
    }
}
