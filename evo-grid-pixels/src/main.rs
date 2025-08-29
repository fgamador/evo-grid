#![deny(clippy::all)]
#![forbid(unsafe_code)]

use evo_grid::EvoWorld;
use pixels_main_support::{animate, window_size_to_grid_size};
use world_grid::Random;

const TIME_STEP_FRAMES: u32 = 60;
const CELL_PIXEL_WIDTH: u32 = 3;

fn main() {
    animate(TIME_STEP_FRAMES, |window_size| {
        EvoWorld::new(
            window_size_to_grid_size(window_size, CELL_PIXEL_WIDTH),
            Random::new(),
        )
    });
}
