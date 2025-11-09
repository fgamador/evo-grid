#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use rand::SeedableRng;
use rand::distr::uniform::{SampleRange, SampleUniform};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use std::fmt::Debug;
use std::mem;
use std::ops::{Index, IndexMut, Range, RangeInclusive};
use std::slice::{ChunksExactMut, Iter, IterMut};

pub trait World {
    fn grid(&self) -> &WorldGrid<impl GridCell>;
    fn update(&mut self);
}

#[derive(Clone, Debug)]
pub struct WorldGrid<C>
where
    C: Clone + GridCell,
{
    size: GridSize,
    pub cells: WorldGridCells<C>,
    pub next_cells: WorldGridCells<C>,
}

impl<C> WorldGrid<C>
where
    C: Clone + Debug + GridCell,
{
    pub fn new(size: GridSize) -> Self {
        assert!(!size.is_empty());
        Self {
            size,
            cells: WorldGridCells::new(size),
            next_cells: WorldGridCells::new(size),
        }
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    pub fn num_cells(&self) -> usize {
        self.cells.num_cells()
    }

    pub fn cell_mut(&mut self, loc: Loc) -> Option<&mut C> {
        self.cells.cell_mut(loc)
    }

    pub fn cells_iter(&self) -> Iter<'_, C> {
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
            .zip(Random::multi_fork_option(rand, self.size.width).par_iter_mut())
            .enumerate()
            .for_each(|(row, (row_next_cells, row_rand))| {
                Self::update_row(
                    row as u32,
                    &self.cells,
                    row_next_cells,
                    self.size.width,
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

    pub fn debug_print(&self, row: u32, col: u32) {
        self.cells[Loc::new(row, col)].debug_print(row, col);
    }
}

#[derive(Clone, Debug)]
pub struct WorldGridCells<C>
where
    C: Clone + GridCell,
{
    size: GridSize,
    cells: Vec<C>,
}

impl<C> WorldGridCells<C>
where
    C: Clone + Copy + Default + GridCell,
{
    pub fn new(size: GridSize) -> Self {
        assert!(!size.is_empty());
        Self {
            size,
            cells: vec![C::default(); size.area()],
        }
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    pub fn num_cells(&self) -> usize {
        self.cells.len()
    }

    pub fn cells_iter(&self) -> Iter<'_, C> {
        self.cells.iter()
    }

    pub fn cells_iter_mut(&mut self) -> IterMut<'_, C> {
        self.cells.iter_mut()
    }

    pub fn rows_mut(&mut self) -> ChunksExactMut<'_, C> {
        self.cells.chunks_exact_mut(self.size.width as usize)
    }

    pub fn par_rows_mut(&mut self) -> rayon::slice::ChunksExactMut<'_, C> {
        self.cells.par_chunks_exact_mut(self.size.width as usize)
    }

    fn cell(&self, loc: Loc) -> Option<&C> {
        loc.grid_index(self.size).map(|index| &self.cells[index])
    }

    fn cell_mut(&mut self, loc: Loc) -> Option<&mut C> {
        loc.grid_index(self.size)
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
    Self: Copy + Debug + Default + Send + Sync,
{
    fn color_rgba(&self) -> [u8; 4];
    fn update(
        &self,
        neighborhood: &Neighborhood<Self>,
        next_cell: &mut Self,
        rand: &mut Option<Random>,
    );
    fn debug_print(&self, row: u32, col: u32);
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
        for row in Self::index_range(self.center.row, self.cells.size.height) {
            for col in Self::index_range(self.center.col, self.cells.size.width) {
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

    pub fn grid_index(&self, size: GridSize) -> Option<usize> {
        if self.row < size.height && self.col < size.width {
            Some((self.row * size.width + self.col) as usize)
        } else {
            None
        }
    }

    pub fn distance(&self, loc: Loc) -> f64 {
        let row_diff = self.row.abs_diff(loc.row);
        let col_diff = self.col.abs_diff(loc.col);
        (((row_diff * row_diff) + (col_diff * col_diff)) as f64).sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSize {
    pub width: u32,
    pub height: u32,
}

impl GridSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn area(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BitSet8Gene {
    pub value: BitSet8,
}

impl BitSet8Gene {
    pub fn new(value: BitSet8) -> Self {
        Self { value }
    }

    pub fn merge(genes: &ArrayVec<Self, 8>, rand: &mut Option<Random>, mutation_odds: f64) -> Self {
        let mut bit_counts = BitCountsMap::new();
        for gene in genes {
            bit_counts.increment(&gene.value);
        }
        Self::new(bit_counts.as_bit_set(rand, mutation_odds))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BitSet8 {
    pub bits: u8,
}

impl BitSet8 {
    pub fn new(bits: u8) -> Self {
        Self { bits }
    }

    pub fn empty() -> Self {
        Self::new(0)
    }

    pub fn random(bit_odds: f64, rand: &mut Random) -> Self {
        let mut result = Self::empty();
        for i in 0..8 {
            if rand.next_bool(bit_odds) {
                result.set_bit(i);
            }
        }
        result
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

    pub fn count_matching_bits(&self, other: Self) -> usize {
        let mismatched_bits = Self::new(self.bits ^ other.bits);
        8 - mismatched_bits.count_set_bits()
    }

    pub fn nybbles(&self) -> (u8, u8) {
        (self.bits & 0xf0, (self.bits & 0x0f) << 4)
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

    fn increment(&mut self, bits: &BitSet8) {
        for i in 0..8 {
            if bits.is_bit_set(i) {
                self.ones[i] += 1;
            } else {
                self.zeros[i] += 1;
            }
        }
    }

    fn as_bit_set(&self, rand: &mut Option<Random>, mutation_odds: f64) -> BitSet8 {
        let mut result = BitSet8::empty();
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

#[derive(Clone, Copy, Debug, Default)]
pub struct FractionGene {
    pub value: f32,
}

impl FractionGene {
    pub fn new(value: f32) -> Self {
        debug_assert!((0.0..=1.0).contains(&value));
        Self { value }
    }

    pub fn merge(
        genes: &ArrayVec<Self, 8>,
        rand: &mut Option<Random>,
        mutation_stdev: f64,
    ) -> Self {
        // do better than just averaging?
        let average_value = genes.iter().map(|gene| gene.value).sum::<f32>() / genes.len() as f32;
        if let Some(rand) = rand {
            Self::new(
                rand.next_truncated_normal(average_value as f64, mutation_stdev, 0.0..=1.0) as f32,
            )
        } else {
            Self::new(average_value)
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

    pub fn next_normal(&mut self, mean: f64, stdev: f64) -> f64 {
        let distr = Normal::new(mean, stdev).unwrap();
        distr.sample(&mut self.rng)
    }

    pub fn next_truncated_normal(
        &mut self,
        mean: f64,
        stdev: f64,
        range: RangeInclusive<f64>,
    ) -> f64 {
        loop {
            let sample = self.next_normal(mean, stdev);
            if range.contains(&sample) {
                return sample;
            }
        }
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

impl Default for Random {
    fn default() -> Self {
        Self::new()
    }
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
