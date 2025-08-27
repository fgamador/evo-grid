#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::slice::Iter;
use world_grid::{
    alpha_blend, GridCell, Loc, Neighborhood, Random, World, WorldGrid, WorldGridCells,
};

#[derive(Debug)]
pub struct EvoWorld {
    grid: WorldGrid<EvoGridCell>,
    sources: Vec<SubstanceSource>,
    rand: Option<Random>,
}

impl EvoWorld {
    pub fn new(width: u32, height: u32, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_substances();
        result.add_creatures();
        // result.cells[(1 + height / 4, width / 2)].debug_selected = true;
        result
    }

    fn new_empty(width: u32, height: u32, rand: Random) -> Self {
        assert!(width != 0 && height != 0);
        Self {
            grid: WorldGrid::new(width, height),
            sources: vec![],
            rand: Some(rand),
        }
    }

    fn add_substances(&mut self) {
        self.add_substance_source_clusters(40, 5, 10);
    }

    fn add_substance_source_clusters(&mut self, count: usize, radius: u32, size: u32) {
        for _ in 0..count {
            let row_range = radius..(self.height() - radius);
            let row = self.rand.as_mut().unwrap().next_in_range(row_range);

            let col_range = radius..(self.width() - radius);
            let col = self.rand.as_mut().unwrap().next_in_range(col_range);

            self.add_substance_source_cluster(Loc::new(row, col), radius, size);
        }
    }

    fn add_substance_source_cluster(&mut self, center: Loc, radius: u32, size: u32) {
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
        let rand = self.rand.as_mut().unwrap();
        let result = [
            0xff,
            rand.next_in_range(0..0xff),
            rand.next_in_range(0..0x80),
        ];
        rand.shuffle_color_rgb(result)
    }

    fn random_offset(&mut self, index: u32, max_offset: u32) -> u32 {
        let rand = self.rand.as_mut().unwrap();
        let offset_range = -(max_offset as i32)..max_offset as i32;
        (index as i32 + rand.next_in_range(offset_range)) as u32
    }

    fn add_creatures(&mut self) {
        let loc = Loc::new(20 + self.height() / 4, self.width() / 3);
        self.grid.cells[loc].creature = Some(Creature::new([0, 0xff, 0]));
    }
}

impl World for EvoWorld {
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
        self.grid.update(&mut self.rand, |grid| {
            self.sources
                .iter()
                .for_each(|source| source.update_cells(&mut grid.next_cells));
        });
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

    fn update_cells(&self, cells: &mut WorldGridCells<EvoGridCell>) {
        let substance = cells[self.loc].substance.get_or_insert_default();
        *substance = self.substance;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoGridCell {
    pub creature: Option<Creature>,
    pub substance: Option<Substance>,
}

impl EvoGridCell {
    fn update_next_creature(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
    ) {
        if let Some(creature) = self.creature {
            creature.update_next_cell(neighborhood, next_cell);
        }
    }

    fn update_next_substance(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
    ) {
        if let Some(substance) = self.substance {
            substance.update_next_cell(neighborhood, next_cell);
        }
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

impl GridCell for EvoGridCell {
    fn color_rgba(&self) -> [u8; 4] {
        alpha_blend(self.render_substance(), self.render_creature())
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
        _rand: &mut Option<Random>,
    ) {
        self.update_next_creature(neighborhood, next_cell);
        self.update_next_substance(neighborhood, next_cell);
    }
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

    fn update_next_cell(
        &self,
        _neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
    ) {
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

    fn update_next_cell(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
    ) {
        let next_substance = next_cell.substance.as_mut().unwrap();

        next_substance.amount += Self::sum_donations(neighborhood, self.color);

        if next_substance.amount < Self::MIN_AMOUNT {
            next_cell.substance = None;
        } else {
            next_substance.amount -= (Self::DONATE_FRACTION + Self::DECAY_FRACTION) * self.amount;
        }
    }

    fn sum_donations(neighborhood: &Neighborhood<EvoGridCell>, color: [u8; 3]) -> f32 {
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
