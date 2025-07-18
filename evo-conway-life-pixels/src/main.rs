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
                let loc = Loc::new(row, col);
                self.grid.cells[loc].alive = self.rand.next_bool(0.3);
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
    pub alive: bool,
    pub debug_selected: bool,
}

impl EvoConwayGridCell {
    fn num_live_neighbors(neighborhood: &Neighborhood<EvoConwayGridCell>) -> u32 {
        let mut result = 0;
        neighborhood.for_neighbor_cells(|neighbor| {
            if neighbor.alive {
                result += 1;
            }
        });
        result
    }
}

impl GridCell for EvoConwayGridCell {
    fn debug_selected(&self) -> bool {
        self.debug_selected
    }

    fn color_rgba(&self) -> [u8; 4] {
        if self.alive {
            [0, 0, 0, 0xff]
        } else {
            [0xff, 0xff, 0xff, 0xff]
        }
    }

    fn update(&self, neighborhood: &Neighborhood<EvoConwayGridCell>, next_cell: &mut EvoConwayGridCell) {
        let neighbors = Self::num_live_neighbors(neighborhood);
        next_cell.alive = if self.alive {
            2 <= neighbors && neighbors <= 3
        } else {
            neighbors == 3
        };
    }
}
