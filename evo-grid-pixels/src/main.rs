#![deny(clippy::all)]
#![forbid(unsafe_code)]

use evo_grid::EvoWorld;
use pixels_main_support::animate;
use world_grid::{Random, GridSize};

const TIME_STEP_FRAMES: u32 = 60;
const CELL_PIXEL_WIDTH: u32 = 3;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        EvoWorld::new(
            GridSize::new(
                window_size.width / CELL_PIXEL_WIDTH,
                window_size.height / CELL_PIXEL_WIDTH,
            ),
            Random::new(),
        )
    });
}
