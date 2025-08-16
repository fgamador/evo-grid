#![deny(clippy::all)]
#![forbid(unsafe_code)]

use itertools::izip;
use pixels::wgpu::Color;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Cursor, CursorIcon, Fullscreen, Window, WindowId};
use world_grid::{alpha_blend, GridCell, World};

const TIME_STEP_FRAMES: u32 = 60;
const BACKGROUND_COLOR: Color = Color::BLACK;
const CURSOR_TIMEOUT_MILLIS: u64 = 1000;

pub fn animate<W, F>(build_world: F)
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut AppEventHandler::new(build_world))
        .unwrap();
}

struct AppEventHandler<W, F>
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    build_world: F,
    app: Option<App<W>>,
    paused: bool,
    cursor_position: PhysicalPosition<f64>,
    cursor_timeout: Option<Instant>,
}

impl<W, F> AppEventHandler<W, F>
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    fn new(build_world: F) -> Self {
        Self {
            build_world,
            app: None,
            paused: false,
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            cursor_timeout: None,
        }
    }

    fn app(&mut self) -> &mut App<W> {
        self.app.as_mut().unwrap()
    }

    fn show_cursor(&mut self) {
        self.app().window.set_cursor_visible(true);
        self.cursor_timeout = Some(Instant::now() + Duration::from_millis(CURSOR_TIMEOUT_MILLIS));
    }

    fn hide_cursor(&mut self) {
        self.app().window.set_cursor_visible(false);
        self.cursor_timeout = None;
    }
}

impl<W, F> ApplicationHandler for AppEventHandler<W, F>
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        if self.app.is_some() && !self.paused {
            self.app().on_frame();
        }

        if let Some(cursor_timeout) = self.cursor_timeout
            && Instant::now() >= cursor_timeout
        {
            self.hide_cursor();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_none() {
            self.app = Some(App::new(event_loop, &self.build_world));
            self.app().on_create();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position;
                self.show_cursor();
            }
            WindowEvent::Focused(true) => {
                self.app().window.request_redraw();
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
                KeyCode::KeyP => {
                    self.paused ^= true;
                }
                KeyCode::KeyS => {
                    self.paused = true;
                    self.app().on_single_step();
                }
                _ => (),
            },
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                let pos = self.cursor_position;
                self.app().on_mouse_click(pos);
                self.show_cursor();
            }
            WindowEvent::RedrawRequested => {
                self.app().draw();
            }
            _ => (),
        }
    }
}

struct App<W: World> {
    world: W,
    window: Arc<Window>,
    pixels: Pixels<'static>,
    cross_fade_buffer: PixelCrossFadeBuffer,
    time_step_frame: u32,
}

impl<W: World> App<W> {
    fn new<F>(event_loop: &ActiveEventLoop, build_world: &F) -> Self
    where
        F: Fn(PhysicalSize<u32>) -> W,
    {
        let window = Arc::new(Self::build_window(event_loop));
        let world = build_world(window.inner_size());
        let pixels = Self::build_pixels(&window, world.width(), world.height());
        let cross_fade_buffer = PixelCrossFadeBuffer::new(world.width(), world.height());
        Self {
            world,
            window,
            pixels,
            cross_fade_buffer,
            time_step_frame: 0,
        }
    }

    fn build_window(event_loop: &ActiveEventLoop) -> Window {
        let window_attributes = Window::default_attributes()
            .with_cursor(Cursor::Icon(CursorIcon::Crosshair))
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_visible(false);
        event_loop.create_window(window_attributes).unwrap()
    }

    fn build_pixels(window: &Arc<Window>, width: u32, height: u32) -> Pixels<'static> {
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());
        PixelsBuilder::new(width, height, surface_texture)
            .clear_color(BACKGROUND_COLOR)
            .build()
            .unwrap()
    }

    fn on_create(&mut self) {
        self.world.update();
        self.cross_fade_buffer.load(self.world.cells_iter());
        self.cross_fade_buffer.blend_to_output(1.0);

        self.window.set_cursor_visible(false);
        self.window.set_visible(true);
        self.window.request_redraw();
    }

    fn on_frame(&mut self) {
        if self.time_step_frame < TIME_STEP_FRAMES {
            self.on_cross_fade_frame();
        } else {
            self.on_time_step_frame();
        }
    }

    fn on_cross_fade_frame(&mut self) {
        self.cross_fade_buffer
            .blend_to_output(self.time_step_frame as f32 / TIME_STEP_FRAMES as f32);
        self.time_step_frame += 1;
        self.window.request_redraw();
    }

    fn on_time_step_frame(&mut self) {
        self.world.update();
        self.cross_fade_buffer.load(self.world.cells_iter());
        self.time_step_frame = 0;
        self.window.request_redraw();
    }

    fn on_single_step(&mut self) {
        self.world.update();
        self.cross_fade_buffer.load(self.world.cells_iter());
        self.cross_fade_buffer.blend_to_output(1.0);
        self.window.request_redraw();
    }

    fn on_mouse_click(&self, pos: PhysicalPosition<f64>) {
        let (col, row) = self
            .pixels
            .window_pos_to_pixel((pos.x as f32, pos.y as f32))
            .unwrap();
        self.world.debug_print(row as u32, col as u32);
    }

    fn draw(&mut self) {
        for (screen_pixel, buffer_pixel) in izip!(
            self.pixels.frame_mut().chunks_exact_mut(4),
            self.cross_fade_buffer.output_pixels.iter()
        ) {
            screen_pixel.copy_from_slice(buffer_pixel);
        }
        self.pixels.render().unwrap();
    }
}

struct PixelCrossFadeBuffer {
    input_pixels: Vec<[u8; 4]>,
    background_pixels: Vec<[u8; 4]>,
    output_pixels: Vec<[u8; 4]>,
}

impl PixelCrossFadeBuffer {
    fn new(width: u32, height: u32) -> Self {
        let num_pixels = (width * height) as usize;
        Self {
            input_pixels: vec![[0; 4]; num_pixels],
            background_pixels: vec![[0; 4]; num_pixels],
            output_pixels: vec![[0; 4]; num_pixels],
        }
    }

    fn load<'a, T: GridCell + 'a>(&mut self, cells: impl Iterator<Item = &'a T>) {
        for (input_pixel, background_pixel, cell) in izip!(
            self.input_pixels.iter_mut(),
            self.background_pixels.iter_mut(),
            cells
        ) {
            *background_pixel = *input_pixel;
            background_pixel[3] = 0xff;

            *input_pixel = cell.color_rgba();
            input_pixel[3] = 0;
        }
    }

    fn blend_to_output(&mut self, fraction: f32) {
        const BLEND_SMOOTHNESS_FACTOR: f32 = 1.5;
        let dark_pixel_alpha = fraction_to_alpha(
            (fraction * BLEND_SMOOTHNESS_FACTOR - (BLEND_SMOOTHNESS_FACTOR - 1.0)).max(0.0),
        );
        let bright_pixel_alpha = fraction_to_alpha((fraction * BLEND_SMOOTHNESS_FACTOR).min(1.0));

        for (input_pixel, background_pixel, output_pixel) in izip!(
            self.input_pixels.iter(),
            self.background_pixels.iter(),
            self.output_pixels.iter_mut()
        ) {
            let mut input_pixel = *input_pixel;
            input_pixel[3] = if is_dark(input_pixel) {
                dark_pixel_alpha
            } else {
                bright_pixel_alpha
            };

            *output_pixel = alpha_blend(input_pixel, *background_pixel);
            output_pixel[3] = 0xff;
        }
    }
}

fn fraction_to_alpha(fraction: f32) -> u8 {
    (fraction * 0xff as f32) as u8
}

fn is_dark(pixel: [u8; 4]) -> bool {
    const MAX_DARK_VALUE: u8 = 0x10;
    pixel[0] <= MAX_DARK_VALUE && pixel[1] <= MAX_DARK_VALUE && pixel[2] <= MAX_DARK_VALUE
}
