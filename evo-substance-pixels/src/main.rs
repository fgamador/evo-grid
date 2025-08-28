#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use pixels_main_support::animate;
use std::fmt::Debug;
use std::slice::Iter;
use world_grid::{
    BitSet8, BitSet8Gene, FractionGene, GridCell, Neighborhood, Random, World, WorldGrid,
};

const TIME_STEP_FRAMES: u32 = 60;
const CELL_PIXEL_WIDTH: u32 = 4;
const EMPTY_CELL_COLOR: [u8; 4] = [0, 0, 0, 0xff];
const MUTATION_ODDS: f64 = 0.001;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        EvoSubstanceWorld::new(
            window_size.width / CELL_PIXEL_WIDTH,
            window_size.height / CELL_PIXEL_WIDTH,
            Random::new(),
        )
    });
}

#[derive(Debug)]
pub struct EvoSubstanceWorld {
    grid: WorldGrid<EvoSubstanceCell>,
    rand: Option<Random>,
}

impl EvoSubstanceWorld {
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
        }
    }

    fn add_random_life(&mut self) {
        for cell in self.grid.cells.cells_iter_mut() {
            if let Some(rand) = self.rand.as_mut()
                && rand.next_bool(0.3)
            {
                // todo
                // cell.creature = Some(Creature::conway());
            }
        }
    }
}

impl World for EvoSubstanceWorld {
    fn width(&self) -> u32 {
        self.grid.width()
    }

    fn height(&self) -> u32 {
        self.grid.height()
    }

    fn num_cells(&self) -> usize {
        self.grid.num_cells()
    }

    fn cells_iter(&self) -> Iter<'_, impl GridCell> {
        self.grid.cells_iter()
    }

    fn update(&mut self) {
        self.grid.update(&mut self.rand, |_grid| {});
    }

    fn debug_print(&self, _row: u32, _col: u32) {}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoSubstanceCell {
    creature: Option<Creature>,
    substance: Option<Substance>,
}

impl EvoSubstanceCell {
    fn num_neighbor_creatures(neighborhood: &Neighborhood<EvoSubstanceCell>) -> usize {
        let mut result = 0;
        neighborhood.for_neighbor_cells(|neighbor| {
            if neighbor.creature.is_some() {
                result += 1;
            }
        });
        result
    }
}

impl GridCell for EvoSubstanceCell {
    fn color_rgba(&self) -> [u8; 4] {
        if let Some(creature) = self.creature {
            creature.color_rgba()
        } else {
            EMPTY_CELL_COLOR
        }
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoSubstanceCell>,
        next_cell: &mut EvoSubstanceCell,
        rand: &mut Option<Random>,
    ) {
        let num_neighbors = Self::num_neighbor_creatures(neighborhood);
        if let Some(creature) = self.creature {
            if !creature.survives(num_neighbors, rand) {
                next_cell.creature = None;
            }
        } else {
            next_cell.creature = Creature::maybe_reproduce(neighborhood, rand);
        };
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Creature {
    enzyme_gene: BitSet8Gene,
    match_weight_gene: FractionGene,
}

impl Creature {
    pub fn new(enzyme_gene: BitSet8Gene, match_weight_gene: FractionGene) -> Self {
        Self {
            enzyme_gene,
            match_weight_gene,
        }
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        // todo
        let red = 0;

        let green = 0;

        let blue = 0;

        [red, green, blue, 0xff]
    }

    pub fn survives(&self, num_neighbors: usize, rand: &mut Option<Random>) -> bool {
        num_neighbors > 0 && self.enzyme_gene.value.is_bit_set(num_neighbors - 1)
        // && self.has_small_genome(rand)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoSubstanceCell>,
        rand: &mut Option<Random>,
    ) -> Option<Creature> {
        if let Some((child_enzyme_gene, child_match_weight_gene)) =
            Self::merge_parent_genes(neighborhood, rand, MUTATION_ODDS)
        {
            let child = Creature::new(child_enzyme_gene, child_match_weight_gene);
            return Some(child);
        }

        None
    }

    fn merge_parent_genes(
        neighborhood: &Neighborhood<EvoSubstanceCell>,
        rand: &mut Option<Random>,
        mutation_odds: f64,
    ) -> Option<(BitSet8Gene, FractionGene)> {
        let mut parent_enzyme_genes = ArrayVec::<BitSet8Gene, 8>::new();
        let mut parent_match_weight_genes = ArrayVec::<FractionGene, 8>::new();
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(creature) = neighbor.creature
            // && creature.can_reproduce(num_neighbors)
            {
                parent_enzyme_genes.push(creature.enzyme_gene);
                parent_match_weight_genes.push(creature.match_weight_gene);
            }
        });

        if parent_enzyme_genes.is_empty() {
            None
        } else {
            Some((
                BitSet8Gene::merge(&parent_enzyme_genes, rand, mutation_odds),
                FractionGene::merge(&parent_match_weight_genes, rand, mutation_odds),
            ))
        }
    }

    fn can_reproduce(&self, num_neighbors: usize) -> bool {
        // todo
        true
        // num_neighbors > 0 && self.match_weight_gene.value.is_bit_set(num_neighbors - 1)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Substance {
    code: BitSet8,
}

impl Substance {
    pub fn new(code: BitSet8) -> Self {
        Self { code }
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        // todo
        let red = 0;

        let green = 0;

        let blue = 0;

        [red, green, blue, 0xff]
    }
}
