#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels_main_support::animate;
use world_grid::{GridCell, Loc, Neighborhood, Random, World, WorldGrid};

const CELL_PIXEL_WIDTH: u32 = 3;

fn main() {
    animate(|window_size| {
        ConwayWorld::new(
            window_size.width / CELL_PIXEL_WIDTH,
            window_size.height / CELL_PIXEL_WIDTH,
            Random::new(),
        )
    });
}

#[derive(Debug)]
pub struct ConwayWorld {
    grid: WorldGrid<ConwayGridCell>,
    rand: Random,
}

impl ConwayWorld {
    pub fn new(width: u32, height: u32, rand: Random) -> Self {
        let mut result = Self::new_empty(width, height, rand);
        result.add_random_life();
        result
    }

    fn new_empty(width: u32, height: u32, rand: Random) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            grid: WorldGrid::new(width, height),
            rand,
        }
    }

    fn add_random_life(&mut self) {
        for row in 0..self.height() {
            for col in 0..self.width() {
                let loc = Loc::new(row, col);
                self.grid.cells[loc].alive = self.rand.next_bool(0.3);
            }
        }
    }
}

impl World for ConwayWorld {
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
        self.grid.update(&mut self.rand, 0.0, |_grid| {});
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

    fn update(
        &self,
        neighborhood: &Neighborhood<ConwayGridCell>,
        next_cell: &mut ConwayGridCell,
        _rand: &mut Random,
        _mutation_odds: f64,
    ) {
        let neighbors = Self::num_live_neighbors(neighborhood);
        next_cell.alive = if self.alive {
            2 <= neighbors && neighbors <= 3
        } else {
            neighbors == 3
        };
    }
}
