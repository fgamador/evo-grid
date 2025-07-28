#![deny(clippy::all)]
#![forbid(unsafe_code)]

use evo_grid::EvoWorld;
use pixels_main_support::animate;
use world_grid::Random;

const CELL_PIXEL_WIDTH: u32 = 3;

fn main() {
    animate(|window_size| {
        EvoWorld::new(
            window_size.width / CELL_PIXEL_WIDTH,
            window_size.height / CELL_PIXEL_WIDTH,
            Random::new(),
        )
    });
}
