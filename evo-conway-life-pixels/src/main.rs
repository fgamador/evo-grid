#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use pixels_main_support::animate;
use std::fmt::Debug;
use world_grid::{BitSet8, GridCell, Loc, Neighborhood, Random, World, WorldGrid};

const TIME_STEP_FRAMES: u32 = 30;
const CELL_PIXEL_WIDTH: u32 = 4;
const EMPTY_CELL_COLOR: [u8; 4] = [0, 0, 0, 0xff];
const MUTATION_ODDS: f64 = 0.001;
const CONWAY_STEPS: usize = 10;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
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
    rand: Option<Random>,
    conway_steps: usize,
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
            rand: Some(rand),
            conway_steps: CONWAY_STEPS,
        }
    }

    fn add_random_life(&mut self) {
        for row in 0..self.height() {
            for col in 0..self.width() {
                if self.rand.as_mut().unwrap().next_bool(0.3) {
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
        if self.conway_steps > 0 {
            self.conway_steps -= 1;
            self.grid.update(&mut None, |_grid| {});
        } else {
            self.grid.update(&mut self.rand, |_grid| {});
        };
    }

    fn debug_print(&self, row: u32, col: u32) {
        self.grid.cells[Loc::new(row, col)].debug_print(row, col);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoConwayGridCell {
    creature: Option<Creature>,
    pub debug_selected: bool,
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

    fn debug_print(&self, row: u32, col: u32) {
        if let Some(creature) = self.creature {
            let color = self.color_rgba();
            println!(
                "({}, {}): Survival: {}, Repro: {}, Color: [0x{:X},0x{:X},0x{:X}]",
                row,
                col,
                Self::format_neighbor_counts(creature.survival_neighbor_counts),
                Self::format_neighbor_counts(creature.repro_neighbor_counts),
                color[0],
                color[1],
                color[2]
            );
        } else {
            println!("({}, {}): No creature", row, col);
        }
    }

    fn format_neighbor_counts(neighbor_counts: BitSet8) -> String {
        let mut result = String::with_capacity(100);
        result.push('[');
        for i in 0..8 {
            if neighbor_counts.has_bit(i) {
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
    fn debug_selected(&self) -> bool {
        self.debug_selected
    }

    fn color_rgba(&self) -> [u8; 4] {
        if let Some(creature) = self.creature {
            creature.color_rgba()
        } else {
            EMPTY_CELL_COLOR
        }
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
}

#[derive(Clone, Copy, Debug, Default)]
struct Creature {
    // bits[n] == 1 means will survive if own cell has n-1 neighbor creatures
    survival_neighbor_counts: BitSet8,
    // bits[n] == 1 means will reproduce if target cell has n-1 neighbor creatures
    repro_neighbor_counts: BitSet8,
}

impl Creature {
    pub fn new(survival_neighbor_counts: BitSet8, repro_neighbor_counts: BitSet8) -> Self {
        Self {
            survival_neighbor_counts,
            repro_neighbor_counts,
        }
    }

    pub fn conway() -> Self {
        Self::new(BitSet8::new(0b110), BitSet8::new(0b100))
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        let counts_bits_union =
            self.survival_neighbor_counts.bits | self.repro_neighbor_counts.bits;
        let red = counts_bits_union; // >> 1 + counts_bits_union >> 2;

        let num_survival_bits = self.survival_neighbor_counts.count_bits() as u8;
        let num_survival_bits_squeezed = (num_survival_bits & 0b1000) | (num_survival_bits << 1);
        let green = num_survival_bits_squeezed << 4;

        let num_repro_bits = self.repro_neighbor_counts.count_bits() as u8;
        let num_repro_bits_squeezed = (num_repro_bits & 0b1000) | (num_repro_bits << 1);
        let blue = num_repro_bits_squeezed << 4;

        [red, green, blue, 0xff]
    }

    pub fn survives(&self, num_neighbors: usize, rand: &mut Option<Random>) -> bool {
        num_neighbors > 0
            && self.survival_neighbor_counts.has_bit(num_neighbors - 1)
            && self.has_small_genome(rand)
    }

    fn has_small_genome(&self, rand: &mut Option<Random>) -> bool {
        if rand.is_none() {
            return true;
        }

        let rand = rand.as_mut().unwrap();
        let num_genome_bits =
            self.survival_neighbor_counts.count_bits() + self.repro_neighbor_counts.count_bits();
        rand.next_bool(1.0 - num_genome_bits as f64 / 16.0)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Option<Random>,
    ) -> Option<Creature> {
        if num_neighbors == 0 {
            return None;
        }

        if let Some((child_survival_gene, child_repro_gene)) =
            Self::merge_parent_genes(neighborhood, num_neighbors, rand, MUTATION_ODDS)
        {
            let child = Creature::new(child_survival_gene, child_repro_gene);
            if child.has_small_genome(rand) {
                Some(child)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn merge_parent_genes(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Option<Random>,
        mutation_odds: f64,
    ) -> Option<(BitSet8, BitSet8)> {
        let mut parent_survival_genes = ArrayVec::<BitSet8, 8>::new();
        let mut parent_repro_genes = ArrayVec::<BitSet8, 8>::new();
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(creature) = neighbor.creature
                && creature.can_reproduce(num_neighbors)
            {
                parent_survival_genes.push(creature.survival_neighbor_counts);
                parent_repro_genes.push(creature.repro_neighbor_counts);
            }
        });

        if parent_survival_genes.is_empty() {
            None
        } else {
            Some((
                BitSet8::merge(&parent_survival_genes, rand, mutation_odds),
                BitSet8::merge(&parent_repro_genes, rand, mutation_odds),
            ))
        }
    }

    fn can_reproduce(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.repro_neighbor_counts.has_bit(num_neighbors - 1)
    }
}
