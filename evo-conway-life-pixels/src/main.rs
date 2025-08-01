#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels_main_support::animate;
use world_grid::{GridCell, Loc, Neighborhood, Random, World, WorldGrid};

const BACKGROUND_COLOR: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
const CELL_PIXEL_WIDTH: u32 = 4;
const MUTATION_ODDS: f64 = 0.01;

fn main() {
    animate(|window_size| {
        EvoConwayWorld::new(
            window_size.width / CELL_PIXEL_WIDTH,
            window_size.height / CELL_PIXEL_WIDTH,
            Random::new(),
        )
    });
}

#[derive(Debug)]
pub struct EvoConwayWorld {
    grid: WorldGrid<EvoConwayGridCell>,
    rand: Random,
}

impl EvoConwayWorld {
    pub fn new(width: u32, height: u32, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_random_life();
        result
    }

    fn new_empty(width: u32, height: u32, rand: Random) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            grid: WorldGrid::new(width, height),
            rand,
        }
    }

    fn add_random_life(&mut self) {
        for row in 0..self.height() {
            for col in 0..self.width() {
                if self.rand.next_bool(0.3) {
                    let loc = Loc::new(row, col);
                    self.grid.cells[loc].creature = Some(Creature::conway());
                }
            }
        }
    }
}

impl World for EvoConwayWorld {
    fn width(&self) -> u32 {
        self.grid.width()
    }

    fn height(&self) -> u32 {
        self.grid.height()
    }

    fn num_cells(&self) -> usize {
        self.grid.num_cells()
    }

    fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &impl GridCell> + Clone {
        self.grid.cells_iter()
    }

    fn update(&mut self) {
        self.grid.update(&mut self.rand, |_grid| {});
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoConwayGridCell {
    creature: Option<Creature>,
    pub debug_selected: bool,
}

impl GridCell for EvoConwayGridCell {
    fn debug_selected(&self) -> bool {
        self.debug_selected
    }

    fn color_rgba(&self) -> [u8; 4] {
        if let Some(creature) = self.creature {
            creature.color_rgba()
        } else {
            BACKGROUND_COLOR
        }
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        next_cell: &mut EvoConwayGridCell,
        rand: &mut Random,
    ) {
        let num_neighbors = Self::num_neighbor_creatures(neighborhood);
        if let Some(creature) = self.creature {
            if !creature.survives(num_neighbors) {
                next_cell.creature = None;
            }
        } else {
            next_cell.creature = Creature::maybe_reproduce(neighborhood, num_neighbors, rand);
        };
    }
}

impl EvoConwayGridCell {
    fn num_neighbor_creatures(neighborhood: &Neighborhood<EvoConwayGridCell>) -> usize {
        let mut result = 0;
        neighborhood.for_neighbor_cells(|neighbor| {
            if neighbor.creature.is_some() {
                result += 1;
            }
        });
        result
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Creature {
    // bits[n] == 1 means will survive if own cell has n-1 neighbor creatures
    survival_neighbor_counts: BitSet8,
    // bits[n] == 1 means will reproduce if target cell has n-1 neighbor creatures
    repro_neighbor_counts: BitSet8,
}

impl Creature {
    pub fn new(survival_neighbor_counts: BitSet8, birth_neighbor_counts: BitSet8) -> Self {
        Self {
            survival_neighbor_counts,
            repro_neighbor_counts: birth_neighbor_counts,
        }
    }

    pub fn conway() -> Self {
        Self::new(BitSet8::new(0b110), BitSet8::new(0b100))
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        let survival_top5 = self.survival_neighbor_counts.bits & 0b11111000;
        let survival_bottom3 = self.survival_neighbor_counts.bits & 0b00000111;
        let repro_top5 = self.repro_neighbor_counts.bits & 0b11111000;
        let repro_bottom3 = self.repro_neighbor_counts.bits & 0b00000111;

        let red = survival_top5;
        let blue = repro_top5;
        let green = (survival_bottom3 << 5) | (repro_bottom3 << 2);

        [red, green, blue, 0xff]
    }

    pub fn survives(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.survival_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn can_reproduce(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.repro_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Random,
    ) -> Option<Creature> {
        if num_neighbors == 0 {
            return None;
        }

        if let Some((survival_bit_counts, birth_bit_counts)) =
            Self::parent_bit_counts(neighborhood, num_neighbors)
        {
            Some(Creature::new(
                survival_bit_counts.as_neighbor_counts(rand),
                birth_bit_counts.as_neighbor_counts(rand),
            ))
        } else {
            None
        }
    }

    fn parent_bit_counts(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
    ) -> Option<(BitCountsMap, BitCountsMap)> {
        let mut has_parents = false;
        let mut survival_bit_counts = BitCountsMap::new();
        let mut birth_bit_counts = BitCountsMap::new();
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(creature) = neighbor.creature {
                if creature.can_reproduce(num_neighbors) {
                    has_parents = true;
                    Self::update_bit_counts(
                        &creature.survival_neighbor_counts,
                        &mut survival_bit_counts,
                    );
                    Self::update_bit_counts(&creature.repro_neighbor_counts, &mut birth_bit_counts);
                }
            }
        });
        if has_parents {
            Some((survival_bit_counts, birth_bit_counts))
        } else {
            None
        }
    }

    fn update_bit_counts(neighbor_counts: &BitSet8, bit_counts: &mut BitCountsMap) {
        for i in 0..8 {
            if neighbor_counts.has_bit(i) {
                bit_counts.add_one(i);
            } else {
                bit_counts.add_zero(i);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct BitSet8 {
    bits: u8,
}

impl BitSet8 {
    pub fn new(bits: u8) -> Self {
        Self { bits }
    }

    pub fn empty() -> Self {
        Self::new(0)
    }

    fn has_bit(&self, index: usize) -> bool {
        self.bits & (1 << index) != 0
    }

    fn set_bit(&mut self, index: usize) {
        self.bits |= 1 << index;
    }

    fn flip_bit(&mut self, index: usize) {
        self.bits ^= 1 << index;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct BitCountsMap {
    ones: [u32; 8],
    zeros: [u32; 8],
}

impl BitCountsMap {
    pub fn new() -> Self {
        Self {
            ones: [0; 8],
            zeros: [0; 8],
        }
    }

    fn add_one(&mut self, index: usize) {
        self.ones[index] += 1;
    }

    fn add_zero(&mut self, index: usize) {
        self.zeros[index] += 1;
    }

    fn num_ones(&self, index: usize) -> usize {
        self.ones[index] as usize
    }

    fn num_zeros(&self, index: usize) -> usize {
        self.zeros[index] as usize
    }

    fn as_neighbor_counts(&self, rand: &mut Random) -> BitSet8 {
        let mut result = BitSet8::empty();
        for i in 0..8 {
            if Self::merge_counts(self.num_ones(i), self.num_zeros(i), rand) {
                result.set_bit(i);
            }
            if rand.next_bool(MUTATION_ODDS) {
                result.flip_bit(i);
            }
        }
        result
    }

    fn merge_counts(num_ones: usize, num_zeros: usize, rand: &mut Random) -> bool {
        if num_ones == 0 {
            false
        } else if num_zeros == 0 {
            true
        } else {
            let odds = num_ones as f64 / (num_ones + num_zeros) as f64;
            rand.next_bool(odds)
        }
    }
}
