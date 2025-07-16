#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::mem;
use world_grid::{alpha_blend, GridCell, Loc, Neighborhood, Random, World, WorldGridCells};

#[derive(Debug)]
pub struct EvoWorld {
    cells: WorldGridCells<EvoGridCell>,
    next_cells: WorldGridCells<EvoGridCell>,
    sources: Vec<SubstanceSource>,
    rand: Random,
}

impl EvoWorld {
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
            cells: WorldGridCells::new(width, height),
            next_cells: WorldGridCells::new(width, height),
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

    fn update_cells(&mut self) {
        self.sources
            .iter()
            .for_each(|source| source.update_cells(&mut self.next_cells));

        for row in 0..self.height() {
            for col in 0..self.width() {
                self.update_cell(Loc::new(row, col));
            }
        }
    }

    fn update_cell(&mut self, loc: Loc) {
        let cell = &self.cells[loc];
        if cell.debug_selected {
            println!("{:?}", cell);
        }

        let neighborhood = Neighborhood::new(&self.cells, loc);
        let next_cell = &mut self.next_cells[loc];
        cell.update(&neighborhood, next_cell);
    }
}

impl World for EvoWorld {
    fn width(&self) -> usize {
        self.cells.width()
    }

    fn height(&self) -> usize {
        self.cells.height()
    }

    fn num_cells(&self) -> usize {
        self.cells.num_cells()
    }

    fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &impl GridCell> + Clone {
        self.cells.cells_iter()
    }

    fn update(&mut self) {
        self.next_cells.copy_from(&self.cells);
        self.update_cells();
        mem::swap(&mut self.next_cells, &mut self.cells);
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

    fn update_cells(&self, grid: &mut WorldGridCells<EvoGridCell>) {
        let substance = grid[self.loc].substance.get_or_insert_default();
        *substance = self.substance;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EvoGridCell {
    pub creature: Option<Creature>,
    pub substance: Option<Substance>,
    pub debug_selected: bool,
}

impl EvoGridCell {
    fn update_next_creature(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
    ) {
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
    fn debug_selected(&self) -> bool {
        self.debug_selected
    }

    fn color_rgba(&self) -> [u8; 4] {
        alpha_blend(self.render_substance(), self.render_creature())
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoGridCell>,
        next_cell: &mut EvoGridCell,
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
