#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::wgpu::Color;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Cursor, CursorIcon, Fullscreen, Window, WindowAttributes, WindowId};
use world_grid::{GridCell, Loc, Neighborhood, Random, World, WorldGrid};

const WIDTH: u32 = 2560 / 3;
const HEIGHT: u32 = 1440 / 3;
const MUTATION_ODDS: f64 = 0.0;

// fn main() -> Result<(), Error> {
//     env_logger::init();
//     let mut world = EvoConwayWorld::new(WIDTH, HEIGHT, Random::new());
//     animate(&mut world)
// }

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    // event_loop.set_control_flow(ControlFlow::Poll);

    // ControlFlow::Wait pauses the event loop if no events are available to process.
    // This is ideal for non-game applications that only update in response to user
    // input, and uses significantly less power/CPU time than ControlFlow::Poll.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)
}

#[derive(Default)]
struct App {
    pixels: Option<Pixels>,
    window: Option<Window>,
    world: Option<EvoConwayWorld>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut window_attributes = WindowAttributes::default();
        window_attributes.cursor = Cursor::Icon(CursorIcon::Crosshair);
        window_attributes.fullscreen = Some(Fullscreen::Borderless(None));
        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(window);
        let window = self.window.as_ref().unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = PixelsBuilder::new(WIDTH, HEIGHT, surface_texture)
            .clear_color(Color::WHITE)
            .build();
        self.pixels = Some(pixels.unwrap());

        // TODO get size from window
        let world = EvoConwayWorld::new(WIDTH as usize, HEIGHT as usize, Random::new());
        self.world = Some(world);

        self.world.as_mut().unwrap().update();
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Released,
                        repeat: false,
                        ..
                    },
                ..
            } => match code {
                KeyCode::Escape | KeyCode::KeyQ | KeyCode::KeyX => {
                    event_loop.exit();
                }
                _ => (),
            },
            WindowEvent::RedrawRequested => {
                let world = self.world.as_ref().unwrap();
                let pixels = self.pixels.as_mut().unwrap();
                let screen = pixels.frame_mut();

                debug_assert_eq!(screen.len(), 4 * world.num_cells());

                for (cell, pixel) in world.cells_iter().zip(screen.chunks_exact_mut(4)) {
                    pixel.copy_from_slice(&cell.color_rgba());
                }
                pixels.render().unwrap()
            }
            _ => (),
        }
        self.world.as_mut().unwrap().update();
        self.window.as_ref().unwrap().request_redraw();
    }
}

struct ViewModel<'a, W: World> {
    pub world: &'a mut W,
}

impl<'a, W: World> ViewModel<'a, W> {
    pub fn new(world: &'a mut W) -> Self {
        Self { world }
    }

    pub fn update(&mut self) {
        self.world.update();
    }

    pub fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.world.num_cells());
        for (cell, pixel) in self.world.cells_iter().zip(screen.chunks_exact_mut(4)) {
            pixel.copy_from_slice(&cell.color_rgba());
        }
    }
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
                    let loc = Loc::new(row as usize, col as usize);
                    self.grid.cells[loc].creature = Some(Creature::conway());
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
        self.grid.update(&mut self.rand, |_grid| {});
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
        if let Some(creature) = self.creature {
            creature.color_rgba()
        } else {
            [0xff, 0xff, 0xff, 0xff]
        }
    }

    fn update(
        &self,
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        next_cell: &mut EvoConwayGridCell,
        rand: &mut Random,
    ) {
        let num_neighbors = Self::num_neighbor_creatures(neighborhood);
        if let Some(creature) = self.creature {
            if !creature.survives(num_neighbors) {
                next_cell.creature = None;
            }
        } else {
            next_cell.creature = Creature::maybe_reproduce(neighborhood, num_neighbors, rand);
        };
    }
}

impl EvoConwayGridCell {
    fn num_neighbor_creatures(neighborhood: &Neighborhood<EvoConwayGridCell>) -> usize {
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
    // bits[n] == 1 means will survive if own cell has n-1 neighbor creatures
    survival_neighbor_counts: BitSet8,
    // bits[n] == 1 means will reproduce if target cell has n-1 neighbor creatures
    birth_neighbor_counts: BitSet8,
}

impl Creature {
    pub fn new(survival_neighbor_counts: BitSet8, birth_neighbor_counts: BitSet8) -> Self {
        Self {
            survival_neighbor_counts,
            birth_neighbor_counts,
        }
    }

    pub fn conway() -> Self {
        Self::new(BitSet8::new(0b110), BitSet8::new(0b100))
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        let red = self.survival_neighbor_counts.bits << 5;
        let blue = self.birth_neighbor_counts.bits << 5;
        [red, 0x00, blue, 0xff]
    }

    pub fn survives(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.survival_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn can_reproduce(&self, num_neighbors: usize) -> bool {
        num_neighbors > 0 && self.birth_neighbor_counts.has_bit(num_neighbors - 1)
    }

    pub fn maybe_reproduce(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
        rand: &mut Random,
    ) -> Option<Creature> {
        if num_neighbors == 0 {
            return None;
        }

        if let Some((survival_bit_counts, birth_bit_counts)) =
            Self::parent_bit_counts(neighborhood, num_neighbors)
        {
            Some(Creature::new(
                survival_bit_counts.as_neighbor_counts(rand),
                birth_bit_counts.as_neighbor_counts(rand),
            ))
        } else {
            None
        }
    }

    fn parent_bit_counts(
        neighborhood: &Neighborhood<EvoConwayGridCell>,
        num_neighbors: usize,
    ) -> Option<(BitCountsMap, BitCountsMap)> {
        let mut has_parents = false;
        let mut survival_bit_counts = BitCountsMap::new();
        let mut birth_bit_counts = BitCountsMap::new();
        neighborhood.for_neighbor_cells(|neighbor| {
            if let Some(creature) = neighbor.creature {
                if creature.can_reproduce(num_neighbors) {
                    has_parents = true;
                    Self::update_bit_counts(
                        &creature.survival_neighbor_counts,
                        &mut survival_bit_counts,
                    );
                    Self::update_bit_counts(&creature.birth_neighbor_counts, &mut birth_bit_counts);
                }
            }
        });
        if has_parents {
            Some((survival_bit_counts, birth_bit_counts))
        } else {
            None
        }
    }

    fn update_bit_counts(neighbor_counts: &BitSet8, bit_counts: &mut BitCountsMap) {
        for i in 0..8 {
            if neighbor_counts.has_bit(i) {
                bit_counts.add_one(i);
            } else {
                bit_counts.add_zero(i);
            }
        }
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

    pub fn empty() -> Self {
        Self::new(0)
    }

    fn has_bit(&self, index: usize) -> bool {
        self.bits & (1 << index) != 0
    }

    fn set_bit(&mut self, index: usize) {
        self.bits |= 1 << index;
    }

    fn flip_bit(&mut self, index: usize) {
        self.bits ^= 1 << index;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct BitCountsMap {
    ones: [u32; 8],
    zeros: [u32; 8],
}

impl BitCountsMap {
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

    fn as_neighbor_counts(&self, rand: &mut Random) -> BitSet8 {
        let mut result = BitSet8::empty();
        for i in 0..8 {
            if Self::merge_counts(self.num_ones(i), self.num_zeros(i), rand) {
                result.set_bit(i);
            }
            if rand.next_bool(MUTATION_ODDS) {
                result.flip_bit(i);
            }
        }
        result
    }

    fn merge_counts(num_ones: usize, num_zeros: usize, rand: &mut Random) -> bool {
        if num_ones == 0 {
            false
        } else if num_zeros == 0 {
            true
        } else {
            let odds = num_ones as f64 / (num_ones + num_zeros) as f64;
            rand.next_bool(odds)
        }
    }
}
