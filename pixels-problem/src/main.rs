use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct App {
    inner: Option<AppInner>,
}

struct AppInner {
    window: Arc<Window>,
    pixels: Pixels<'static>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let surface_texture = SurfaceTexture::new(100, 100, window.clone());
        let pixels = PixelsBuilder::new(100, 100, surface_texture)
            .build()
            .unwrap();
        self.inner = Some(AppInner { window, pixels });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let inner = self.inner.as_ref().unwrap();
                // Draw using self.pixels...
                inner.pixels.render().unwrap();
                // Show that we can still use the window.
                inner.window.request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
