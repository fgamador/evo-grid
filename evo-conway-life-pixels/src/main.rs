#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use pixels_main_support::{animate, window_size_to_grid_size};
use std::fmt::Debug;
use world_grid::{
    BitSet8, BitSet8Gene, GridCell, GridSize, Neighborhood, Random, World, WorldGrid,
};

const TIME_STEP_FRAMES: u32 = 20;
const CELL_PIXEL_WIDTH: u32 = 4;
const EMPTY_CELL_COLOR: [u8; 4] = [0, 0, 0, 0xff];
const MUTATION_ODDS: f64 = 0.001;
const CONWAY_STEPS: usize = 30;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        EvoConwayWorld::new(
            window_size_to_grid_size(window_size, CELL_PIXEL_WIDTH),
            Random::new(),
        )
    });
}

#[derive(Debug)]
pub struct EvoConwayWorld {
    grid: WorldGrid<EvoConwayGridCell>,
    rand: Option<Random>,
    conway_steps: usize,
}

impl EvoConwayWorld {
    pub fn new(grid_size: GridSize, rand: Random) -> Self {
        let mut result = Self::new_empty(grid_size, rand);
        result.add_random_life();
        result
    }

    fn new_empty(grid_size: GridSize, rand: Random) -> Self {
        assert!(!grid_size.is_empty());
        Self {
            grid: WorldGrid::new(grid_size),
            rand: Some(rand),
            conway_steps: CONWAY_STEPS,
        }
    }

    fn add_random_life(&mut self) {
        for cell in self.grid.cells.cells_iter_mut() {
            if let Some(rand) = self.rand.as_mut()
                && rand.next_bool(0.3)
            {
                cell.creature = Some(Creature::conway());
            }
        }
    }
}

impl World for EvoConwayWorld {
    fn grid(&self) -> &WorldGrid<impl GridCell> {
        &self.grid
    }

    fn update(&mut self) {
        if self.conway_steps > 0 {
            self.conway_steps -= 1;
            self.grid.update(&mut None, |_grid| {});
        } else {
            self.grid.update(&mut self.rand, |_grid| {});
        };
    }

    fn reset(&mut self) {
        self.grid.clear();
        self.add_random_life();
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoConwayGridCell {
    creature: Option<Creature>,
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

    fn format_neighbor_count_gene(neighbor_counts: BitSet8Gene) -> String {
        let mut result = String::with_capacity(100);
        result.push('[');
        for i in 0..8 {
            if neighbor_counts.value.is_bit_set(i) {
                if result.len() > 1 {
                    result.push(',');
                }
                result.push_str(&format!("{}", i + 1));
            }
        }
        result.push(']');
        result
    }
}

impl GridCell for EvoConwayGridCell {
    fn color_rgba(&self) -> [u8; 4] {
        if let Some(creature) = self.creature {
            creature.color_rgba()
        } else {
            EMPTY_CELL_COLOR
        }
    }

    fn clear(&mut self) {
        self.creature = None;
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        next_cell: &mut EvoConwayGridCell,
        rand: &mut Option<Random>,
    ) {
        let num_neighbors = Self::num_neighbor_creatures(neighborhood);
        if let Some(creature) = self.creature {
            if !creature.survives(num_neighbors, rand) {
                next_cell.creature = None;
            }
        } else {
            next_cell.creature = Creature::maybe_reproduce(neighborhood, num_neighbors, rand);
        };
    }

    fn debug_print(&self, row: u32, col: u32) {
        if let Some(creature) = self.creature {
            let color = self.color_rgba();
            println!(
                "({}, {}): Survival: {}, Repro: {}, Color: [0x{:X},0x{:X},0x{:X}]",
                row,
                col,
                Self::format_neighbor_count_gene(creature.survival_gene),
                Self::format_neighbor_count_gene(creature.repro_gene),
                color[0],
                color[1],
                color[2]
            );
        } else {
            println!("({}, {}): No creature", row, col);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Creature {
    // bits[n] == 1 means will survive if own cell has n-1 neighbor creatures
    survival_gene: BitSet8Gene,
    // bits[n] == 1 means will reproduce if target cell has n-1 neighbor creatures
    repro_gene: BitSet8Gene,
}

impl Creature {
    pub fn new(survival_gene: BitSet8Gene, repro_gene: BitSet8Gene) -> Self {
        Self {
            survival_gene,
            repro_gene,
        }
    }

    pub fn conway() -> Self {
        Self::new(
            BitSet8Gene::new(BitSet8::new(0b110)),
            BitSet8Gene::new(BitSet8::new(0b100)),
        )
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        let survival_bitset = self.survival_gene.value;
        let repro_bitset = self.repro_gene.value;

        let counts_bits_union = survival_bitset.bits | repro_bitset.bits;
        let red = counts_bits_union; // >> 1 + counts_bits_union >> 2;

        let num_survival_bits = survival_bitset.count_set_bits() as u8;
        let num_survival_bits_squeezed = (num_survival_bits & 0b1000) | (num_survival_bits << 1);
        let green = num_survival_bits_squeezed << 4;

        let num_repro_bits = repro_bitset.count_set_bits() as u8;
        let num_repro_bits_squeezed = (num_repro_bits & 0b1000) | (num_repro_bits << 1);
        let blue = num_repro_bits_squeezed << 4;

        [red, green, blue, 0xff]
    }

    pub fn survives(&self, num_neighbors: usize, rand: &mut Option<Random>) -> bool {
        num_neighbors > 0
            && self.survival_gene.value.is_bit_set(num_neighbors - 1)
            && self.has_small_genome(rand)
    }

    fn has_small_genome(&self, rand: &mut Option<Random>) -> bool {
        if let Some(rand) = rand {
            let num_genome_bits =
                self.survival_gene.value.count_set_bits() + self.repro_gene.value.count_set_bits();
            rand.next_bool(1.0 - num_genome_bits as f64 / 16.0)
        } else {
            true
        }
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Option<Random>,
    ) -> Option<Creature> {
        if num_neighbors > 0
            && let Some((child_survival_gene, child_repro_gene)) =
                Self::merge_parent_genes(neighborhood, num_neighbors, rand, MUTATION_ODDS)
        {
            let child = Creature::new(child_survival_gene, child_repro_gene);
            if child.has_small_genome(rand) {
                return Some(child);
            }
        }

        None
    }

    fn merge_parent_genes(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Option<Random>,
        mutation_odds: f64,
    ) -> Option<(BitSet8Gene, BitSet8Gene)> {
        let mut parent_survival_genes = ArrayVec::<BitSet8Gene, 8>::new();
        let mut parent_repro_genes = ArrayVec::<BitSet8Gene, 8>::new();
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(creature) = neighbor.creature
                && creature.can_reproduce(num_neighbors)
            {
                parent_survival_genes.push(creature.survival_gene);
                parent_repro_genes.push(creature.repro_gene);
            }
        });

        if parent_survival_genes.is_empty() {
            None
        } else {
            Some((
                BitSet8Gene::merge(&parent_survival_genes, rand, mutation_odds),
                BitSet8Gene::merge(&parent_repro_genes, rand, mutation_odds),
            ))
        }
    }

    fn can_reproduce(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.repro_gene.value.is_bit_set(num_neighbors - 1)
    }
}
