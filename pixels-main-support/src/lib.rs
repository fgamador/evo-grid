#![deny(clippy::all)]
#![forbid(unsafe_code)]

use pixels::wgpu::Color;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Cursor, CursorIcon, Fullscreen, Window, WindowId};
use world_grid::{GridCell, World};

const TIME_STEP_MILLIS: u64 = 400;
const BACKGROUND_COLOR: Color = Color::WHITE;

pub fn animate<W, F>(build_world: F)
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut AppEventHandler::new(build_world))
        .unwrap();
}

struct App<W: World> {
    world: W,
    window: Arc<Window>,
    pixels: Pixels<'static>,
    next_update: Instant,
}

impl<W: World> App<W> {
    fn new<F>(event_loop: &ActiveEventLoop, build_world: &F) -> Self
    where
        F: Fn(PhysicalSize<u32>) -> W,
    {
        let window = Arc::new(Self::build_window(event_loop));
        let world = build_world(window.inner_size());
        let pixels = Self::build_pixels(&window, world.width(), world.height());
        Self {
            world,
            window,
            pixels,
            next_update: Instant::now(),
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
        self.window.request_redraw();
        self.window.set_visible(true);
    }

    fn on_time_step(&mut self) {
        self.world.update();
        self.window.request_redraw();

        while self.next_update < Instant::now() {
            self.next_update += Duration::from_millis(TIME_STEP_MILLIS);
        }
    }

    fn on_redraw(&mut self) {
        let screen = self.pixels.frame_mut();
        debug_assert_eq!(screen.len(), 4 * self.world.num_cells());

        for (cell, pixel) in self.world.cells_iter().zip(screen.chunks_exact_mut(4)) {
            pixel.copy_from_slice(&cell.color_rgba());
        }
        self.pixels.render().unwrap();
    }
}

struct AppEventHandler<W, F>
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    build_world: F,
    app: Option<App<W>>,
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
        }
    }

    fn app(&mut self) -> &mut App<W> {
        self.app.as_mut().unwrap()
    }
}

impl<W, F> ApplicationHandler for AppEventHandler<W, F>
where
    W: World,
    F: Fn(PhysicalSize<u32>) -> W,
{
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            self.app().on_time_step();
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
                self.app().on_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let wakeup_time = self.app().next_update;
        event_loop.set_control_flow(ControlFlow::WaitUntil(wakeup_time));
    }
}
