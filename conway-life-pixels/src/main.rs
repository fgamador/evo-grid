#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels_main_support::{animate, window_size_to_grid_size};
use world_grid::{GridCell, Neighborhood, Random, GridSize, World, WorldGrid};

const TIME_STEP_FRAMES: u32 = 4;
const CELL_PIXEL_WIDTH: u32 = 4;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        ConwayWorld::new(
            window_size_to_grid_size(window_size, CELL_PIXEL_WIDTH),
            Random::new(),
        )
    });
}

#[derive(Debug)]
pub struct ConwayWorld {
    grid: WorldGrid<ConwayGridCell>,
    rand: Option<Random>,
}

impl ConwayWorld {
    pub fn new(grid_size: GridSize, rand: Random) -> Self {
        let mut result = Self::new_empty(grid_size, rand);
        result.add_random_life();
        for _ in 0..5 {
            result.update();
        }
        result
    }

    fn new_empty(grid_size: GridSize, rand: Random) -> Self {
        assert!(!grid_size.is_empty());
        Self {
            grid: WorldGrid::new(grid_size),
            rand: Some(rand),
        }
    }

    fn add_random_life(&mut self) {
        for cell in self.grid.cells.cells_iter_mut() {
            if let Some(rand) = self.rand.as_mut()
                && rand.next_bool(0.3)
            {
                cell.alive = true;
            }
        }
    }
}

impl World for ConwayWorld {
    fn grid(&self) -> &WorldGrid<impl GridCell> {
        &self.grid
    }

    fn update(&mut self) {
        self.grid.update(&mut self.rand, |_grid| {});
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ConwayGridCell {
    pub alive: bool,
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
    fn color_rgba(&self) -> [u8; 4] {
        if self.alive {
            [0x80, 0x80, 0x80, 0xff]
        } else {
            [0x00, 0x00, 0x40, 0xff]
        }
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<ConwayGridCell>,
        next_cell: &mut ConwayGridCell,
        _rand: &mut Option<Random>,
    ) {
        let neighbors = Self::num_live_neighbors(neighborhood);
        next_cell.alive = if self.alive {
            (2..=3).contains(&neighbors)
        } else {
            neighbors == 3
        };
    }

    fn debug_print(&self, _row: u32, _col: u32) {}
}
