#![deny(clippy::all)]
#![forbid(unsafe_code)]

use evo_grid::world::EvoWorld;
use pixels::Error;
use pixels_main_support::animate;
use world_grid::Random;

const WIDTH: usize = 400;
const HEIGHT: usize = 300;

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut world = EvoWorld::new(WIDTH, HEIGHT, Random::new());
    animate(&mut world)
}
