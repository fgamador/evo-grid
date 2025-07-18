#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::Error;
use pixels_main_support::animate;
use world_grid::{GridCell, Loc, Neighborhood, Random, World, WorldGrid};

const WIDTH: usize = 400;
const HEIGHT: usize = 300;

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut world = EvoConwayWorld::new(WIDTH, HEIGHT, Random::new());
    animate(&mut world)
}

#[derive(Debug)]
pub struct EvoConwayWorld {
    grid: WorldGrid<EvoConwayGridCell>,
    rand: Random,
}

impl EvoConwayWorld {
    pub fn new(width: usize, height: usize, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_random_life();
        result
    }

    fn new_empty(width: usize, height: usize, rand: Random) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            grid: WorldGrid::new(width, height),
            rand,
        }
    }

    fn add_random_life(&mut self) {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if self.rand.next_bool(0.3) {
                    let loc = Loc::new(row, col);
                    self.grid.cells[loc].creature = Some(Creature::new());
                }
            }
        }
    }
}

impl World for EvoConwayWorld {
    fn width(&self) -> usize {
        self.grid.width()
    }

    fn height(&self) -> usize {
        self.grid.height()
    }

    fn num_cells(&self) -> usize {
        self.grid.num_cells()
    }

    fn cells_iter(&self) -> impl DoubleEndedIterator<Item = &impl GridCell> + Clone {
        self.grid.cells_iter()
    }

    fn update(&mut self) {
        self.grid.update(|_grid| {});
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
        if self.creature.is_some() {
            [0, 0, 0, 0xff]
        } else {
            [0xff, 0xff, 0xff, 0xff]
        }
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        next_cell: &mut EvoConwayGridCell,
    ) {
        let num_neighbors = Self::num_live_neighbors(neighborhood);
        if let Some(creature) = self.creature {
            if !creature.survives(num_neighbors) {
                next_cell.creature = None;
            }
        } else {
            if Creature::born(num_neighbors) {
                next_cell.creature = Some(Creature::new());
            }
        };
    }
}

impl EvoConwayGridCell {
    fn num_live_neighbors(neighborhood: &Neighborhood<EvoConwayGridCell>) -> usize {
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
    survival_neighbor_counts: BitSet8,
    // birth_neighbor_counts: BitSet8,
}

impl Creature {
    pub fn new() -> Self {
        Self {
            survival_neighbor_counts: BitSet8::new(0b110),
            // birth_neighbor_counts: BitSet8::new(0b100),
        }
    }

    pub fn survives(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.survival_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn born(num_neighbors: usize) -> bool {
        num_neighbors == 3
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

    fn has_bit(&self, index: usize) -> bool {
        self.bits & (1 << index) != 0
    }
}
