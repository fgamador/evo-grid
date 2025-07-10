#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use evo_grid::world::{Creature, GridCell, Substance, World};
use log::{/* debug, */ error};
use pixels::wgpu::Color;
use pixels::{Error, Pixels, PixelsBuilder, SurfaceTexture};
use winit::window::Window;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut world = World::new(
        WIDTH as usize,
        HEIGHT as usize,
        evo_grid::world::Random::new(),
    );
    animate(ViewModel::new(&mut world))
}

struct ViewModel<'a> {
    pub world: &'a mut World,
}

impl<'a> ViewModel<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self { world }
    }

    pub fn update(&mut self) {
        self.world.update();
    }

    pub fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.world.num_cells());
        for (cell, pixel) in self.world.cells_iter().zip(screen.chunks_exact_mut(4)) {
            let color_rgba = Self::render_cell(cell);
            pixel.copy_from_slice(&color_rgba);
        }
    }

    fn render_cell(cell: &GridCell) -> [u8; 4] {
        alpha_blend(
            Self::render_cell_substance(cell.substance),
            Self::render_cell_creature(cell.creature),
        )
    }

    fn render_cell_creature(cell_creature: Option<Creature>) -> [u8; 4] {
        if let Some(creature) = cell_creature {
            let color_rgb = creature.color;
            [color_rgb[0], color_rgb[1], color_rgb[2], 0xff]
        } else {
            [0, 0, 0, 0]
        }
    }

    fn render_cell_substance(cell_substance: Option<Substance>) -> [u8; 4] {
        if let Some(substance) = cell_substance {
            let color_rgb = substance.color;
            let color_alpha = (substance.amount * 0xff as f32) as u8; // .max(0x99);
            [color_rgb[0], color_rgb[1], color_rgb[2], color_alpha]
        } else {
            [0, 0, 0, 0]
        }
    }
}

fn animate(mut view_model: ViewModel) -> Result<(), Error> {
    let event_loop = EventLoop::new().unwrap();
    let window = build_window(&event_loop);
    let mut pixels = build_pixels(&window)?;

    let mut input = WinitInputHelper::new();
    let mut paused = false;

    let res = event_loop.run(|event, elwt| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            view_model.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                elwt.exit();
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }
            if input.key_pressed(KeyCode::KeyP) {
                paused = !paused;
            }
            if input.key_pressed_os(KeyCode::Space) {
                // Space is frame-step, so ensure we're paused
                paused = true;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    elwt.exit();
                    return;
                }
            }
            if !paused || input.key_pressed_os(KeyCode::Space) {
                view_model.update();
            }
            window.request_redraw();
        }
    });
    res.map_err(|e| Error::UserDefined(Box::new(e)))
}

fn build_window(event_loop: &EventLoop<()>) -> Window {
    let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
    let scaled_size = LogicalSize::new(WIDTH as f64 * 3.0, HEIGHT as f64 * 3.0);
    WindowBuilder::new()
        .with_title("Evo")
        .with_inner_size(scaled_size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap()
}

fn build_pixels(window: &Window) -> Result<Pixels, Error> {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    PixelsBuilder::new(WIDTH, HEIGHT, surface_texture)
        .clear_color(Color::WHITE)
        .build()
}

// From https://en.wikipedia.org/wiki/Alpha_compositing
fn alpha_blend(above: [u8; 4], below: [u8; 4]) -> [u8; 4] {
    let above = color_as_fractions(above);
    let below = color_as_fractions(below);

    let above_alpha = above[3];
    let below_alpha = below[3];
    let result_alpha = above_alpha + below_alpha * (1.0 - above_alpha);

    let mut result: [f32; 4] = [0.0, 0.0, 0.0, result_alpha];
    for i in 0..=2 {
        result[i] =
            (above[i] * above_alpha + below[i] * below_alpha * (1.0 - above_alpha)) / result_alpha;
    }
    color_as_bytes(result)
}

fn color_as_fractions(color: [u8; 4]) -> [f32; 4] {
    let mut result: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
    for i in 0..=3 {
        result[i] = color[i] as f32 / 0xff as f32;
    }
    result
}

fn color_as_bytes(color: [f32; 4]) -> [u8; 4] {
    let mut result: [u8; 4] = [0, 0, 0, 0];
    for i in 0..=3 {
        result[i] = (color[i] * 0xff as f32) as u8;
    }
    result
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}
