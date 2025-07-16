#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use evo_grid::world::{EvoWorld, GridCell, World};
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

const WIDTH: usize = 400;
const HEIGHT: usize = 300;

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut world = EvoWorld::new(
        WIDTH,
        HEIGHT,
        evo_grid::world::Random::new(),
    );
    animate(ViewModel::new(&mut world))
}

struct ViewModel<'a, W: World> {
    pub world: &'a mut W,
}

impl<'a, W: World> ViewModel<'a, W> {
    pub fn new(world: &'a mut W) -> Self {
        Self { world }
    }

    fn width(&self) -> usize {
        self.world.width()
    }

    fn height(&self) -> usize {
        self.world.height()
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

fn animate<W: World>(mut view_model: ViewModel<W>) -> Result<(), Error> {
    let event_loop = EventLoop::new().unwrap();
    let window = build_window(view_model.width() as f64, view_model.height() as f64, &event_loop);
    let mut pixels = build_pixels(view_model.width() as u32, view_model.height() as u32, &window)?;

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

fn build_window(width: f64, height: f64, event_loop: &EventLoop<()>) -> Window {
    let size = LogicalSize::new(width, height);
    let scaled_size = LogicalSize::new(width * 3.0, height * 3.0);
    WindowBuilder::new()
        .with_title("Evo")
        .with_inner_size(scaled_size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap()
}

fn build_pixels(width: u32, height: u32, window: &Window) -> Result<Pixels, Error> {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    PixelsBuilder::new(width, height, surface_texture)
        .clear_color(Color::WHITE)
        .build()
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}
