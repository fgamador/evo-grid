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
            next_cell.creature = Creature::maybe_reproduce(neighborhood, num_neighbors);
            // if Creature::born(neighborhood, num_neighbors) {
            //     next_cell.creature = Some(Creature::new());
            // }
        };
    }
}

impl EvoConwayGridCell {
    fn can_reproduce(&self, num_neighbors: usize) -> bool {
        if let Some(creature) = self.creature {
            creature.can_reproduce(num_neighbors)
        } else {
            false
        }
    }

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
    birth_neighbor_counts: BitSet8,
}

impl Creature {
    pub fn new() -> Self {
        Self {
            survival_neighbor_counts: BitSet8::new(0b110),
            birth_neighbor_counts: BitSet8::new(0b100),
        }
    }

    pub fn survives(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.survival_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
    ) -> Option<Creature> {
        if num_neighbors == 0 {
            return None;
        }

        if num_neighbors == 3 {
            Some(Creature::new())
        } else {
            None
        }

        // let mut result = false;
        // neighborhood.for_neighbor_cells(|neighbor| {
        //     if neighbor.can_reproduce(num_neighbors) {
        //         result = true;
        //     }
        // });
        // result
    }

    // pub fn born(neighborhood: &Neighborhood<EvoConwayGridCell>, num_neighbors: usize) -> bool {
    //     if num_neighbors == 0 {
    //         return false;
    //     }
    //
    //     let mut result = false;
    //     neighborhood.for_neighbor_cells(|neighbor| {
    //         if neighbor.can_reproduce(num_neighbors) {
    //             result = true;
    //         }
    //     });
    //     result
    // }

    // fn bit_counts(
    //     neighborhood: &Neighborhood<EvoConwayGridCell>,
    //     num_neighbors: usize,
    // ) -> CountsMap8 {
    //     let mut result = CountsMap8::new();
    //     neighborhood.for_neighbor_cells(|neighbor| {
    //         if neighbor.can_reproduce(num_neighbors) {
    //             result = true;
    //         }
    //     });
    //     result
    // }

    fn can_reproduce(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.birth_neighbor_counts.has_bit(num_neighbors - 1)
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

#[derive(Clone, Copy, Debug, Default)]
struct CountsMap8 {
    ones: [u32; 8],
    zeros: [u32; 8],
}

impl CountsMap8 {
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
}
