#![deny(clippy::all)]
#![forbid(unsafe_code)]

use arrayvec::ArrayVec;
use pixels_main_support::{animate, window_size_to_grid_size};
use std::fmt::Debug;
use world_grid::{
    BitSet8, BitSet8Gene, FractionGene, GridCell, GridSize, Loc, Neighborhood, Random, World,
    WorldGrid, alpha_blend_with_background,
};

const TIME_STEP_FRAMES: u32 = 20;
const CELL_PIXEL_WIDTH: u32 = 4;
const EMPTY_CELL_COLOR: [u8; 4] = [0, 0, 0, 0xff];
const MUTATION_ODDS: f64 = 0.001;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        EvoSubstanceWorld::new(
            window_size_to_grid_size(window_size, CELL_PIXEL_WIDTH),
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
    pub fn new(grid_size: GridSize, rand: Random) -> Self {
        let mut result = Self::new_empty(grid_size, rand);
        result.add_random_substances();
        result.add_random_life();
        result
    }

    fn new_empty(grid_size: GridSize, rand: Random) -> Self {
        Self {
            grid: WorldGrid::new(grid_size),
            rand: Some(rand),
        }
    }

    fn add_random_substances(&mut self) {
        for _ in 0..=5 {
            let center = self.random_loc();
            let radius = self.random_blob_radius();
            let substance = self.random_substance();
            self.add_random_substance_blob(center, radius, substance);
        }
    }

    fn random_loc(&mut self) -> Loc {
        let rand = self.rand.as_mut().unwrap();
        let row = rand.next_in_range(0..=self.grid.size().height);
        let col = rand.next_in_range(0..=self.grid.size().width);
        Loc::new(row, col)
    }

    fn random_blob_radius(&mut self) -> u32 {
        let max_radius = self.grid.size().width.min(self.grid.size().height) / 4;
        let rand = self.rand.as_mut().unwrap();
        rand.next_in_range(10..=max_radius)
    }

    fn random_substance(&mut self) -> Substance {
        let rand = self.rand.as_mut().unwrap();
        Substance::new(BitSet8::random(0.5, rand))
    }

    fn add_random_substance_blob(&mut self, center: Loc, radius: u32, substance: Substance) {
        let (upper_left, lower_right) = self.cell_box(center, radius);
        let rand = self.rand.as_mut().unwrap();
        for row in upper_left.row..=lower_right.row {
            for col in upper_left.col..=lower_right.col {
                let loc = Loc::new(row, col);
                let fraction_of_radius = loc.distance(center) / radius as f64;
                if fraction_of_radius < 1.0
                    && rand.next_bool(1.0 - fraction_of_radius)
                    && let Some(cell) = self.grid.cell_mut(loc)
                {
                    cell.substance = Some(substance);
                }
            }
        }
    }

    fn cell_box(&mut self, center: Loc, radius: u32) -> (Loc, Loc) {
        let min_row = center.row.saturating_sub(radius);
        let max_row = (center.row + radius).min(self.grid.size().height - 1);
        let min_col = center.col.saturating_sub(radius);
        let max_col = (center.col + radius).min(self.grid.size().width - 1);
        (Loc::new(min_row, min_col), Loc::new(max_row, max_col))
    }

    fn add_random_life(&mut self) {
        let rand = self.rand.as_mut().unwrap();
        for cell in self.grid.cells.cells_iter_mut() {
            if rand.next_bool(0.1) {
                cell.creature = Some(Self::random_creature(rand));
            }
        }
    }

    fn random_creature(rand: &mut Random) -> Creature {
        let enzyme = BitSet8::random(0.5, rand);
        Creature::new(BitSet8Gene::new(enzyme), FractionGene::new(0.5))
    }
}

impl World for EvoSubstanceWorld {
    fn grid(&self) -> &WorldGrid<impl GridCell> {
        &self.grid
    }

    fn update(&mut self) {
        self.grid.update(&mut self.rand, |_grid| {});
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoSubstanceCell {
    creature: Option<Creature>,
    substance: Option<Substance>,
}

impl GridCell for EvoSubstanceCell {
    fn color_rgba(&self) -> [u8; 4] {
        let mut result = self.substance.map(|substance| substance.color_rgba());
        if let Some(creature) = self.creature {
            let mut creature_color = creature.color_rgba();
            result = result.map_or(Some(creature_color), |color| {
                creature_color[3] = 0x80;
                Some(alpha_blend_with_background(creature_color, color))
            });
        }
        result.unwrap_or(EMPTY_CELL_COLOR)
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoSubstanceCell>,
        next_cell: &mut EvoSubstanceCell,
        rand: &mut Option<Random>,
    ) {
        if let Some(creature) = self.creature {
            if !creature.survives(&self.substance, rand) {
                next_cell.creature = None;
            }
        } else {
            next_cell.creature = Creature::maybe_reproduce(neighborhood, rand);
        };
    }

    fn debug_print(&self, _row: u32, _col: u32) {}
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
        let (high, low) = self.enzyme_gene.value.nybbles();
        let red = high;
        let green = low;
        let blue = 0x10;
        [red, green, blue, 0xff]
    }

    pub fn survives(&self, substance: &Option<Substance>, rand: &mut Option<Random>) -> bool {
        let rand = rand.as_mut().unwrap();
        let odds = substance.map_or(0.8, |substance| {
            substance.match_degree(self.enzyme_gene.value)
        });
        rand.next_bool(odds)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoSubstanceCell>,
        rand: &mut Option<Random>,
    ) -> Option<Creature> {
        if let Some((child_enzyme_gene, child_match_weight_gene)) =
            Self::merge_parent_genes(neighborhood, rand, MUTATION_ODDS)
        {
            Some(Creature::new(child_enzyme_gene, child_match_weight_gene))
        } else {
            None
        }
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
                && creature.can_reproduce()
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

    fn can_reproduce(&self) -> bool {
        // todo
        false
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
        let (high, low) = self.code.nybbles();
        let red = 0x40;
        let green = high >> 1;
        let blue = low >> 1;
        [red, green, blue, 0xff]
    }

    fn match_degree(&self, bits: BitSet8) -> f64 {
        self.code.count_matching_bits(bits) as f64 / 8.0
    }
}
