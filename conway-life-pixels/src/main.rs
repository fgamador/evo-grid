#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::Error;
use pixels_main_support::animate;
use std::mem;
use world_grid::{GridCell, Loc, Neighborhood, Random, World, WorldGridCells};

const WIDTH: usize = 400;
const HEIGHT: usize = 300;

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut world = ConwayWorld::new(WIDTH, HEIGHT, Random::new());
    animate(&mut world)
}

#[derive(Debug)]
pub struct ConwayWorld {
    cells: WorldGridCells<ConwayGridCell>,
    next_cells: WorldGridCells<ConwayGridCell>,
    rand: Random,
}

impl ConwayWorld {
    pub fn new(width: usize, height: usize, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_random_life();
        result
    }

    fn new_empty(width: usize, height: usize, rand: Random) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            cells: WorldGridCells::new(width, height),
            next_cells: WorldGridCells::new(width, height),
            rand,
        }
    }

    fn add_random_life(&mut self) {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                let loc = Loc::new(row, col);
                self.cells[loc].alive = self.rand.next_bool(0.3);
            }
        }
    }

    fn update_cells(&mut self) {
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

impl World for ConwayWorld {
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

#[derive(Clone, Copy, Debug, Default)]
pub struct ConwayGridCell {
    pub alive: bool,
    pub debug_selected: bool,
}

impl ConwayGridCell {
    fn num_live_neighbors(neighborhood: &Neighborhood<ConwayGridCell>) -> u32 {
        let mut result = 0;
        neighborhood.for_neighbor_cells(|neighbor| {
            if neighbor.alive {
                result += 1;
            }
        });
        result
    }
}

impl GridCell for ConwayGridCell {
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

    fn update(&self, neighborhood: &Neighborhood<ConwayGridCell>, next_cell: &mut ConwayGridCell) {
        let neighbors = Self::num_live_neighbors(neighborhood);
        next_cell.alive = if self.alive {
            2 <= neighbors && neighbors <= 3
        } else {
            neighbors == 3
        };
    }
}
