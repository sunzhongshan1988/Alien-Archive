use std::time::Instant;

use anyhow::{Error, Result};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::{Camera2d, InputState, Renderer, renderer::WgpuRenderer};

pub trait Game {
    fn setup(&mut self, _renderer: &mut dyn Renderer) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, dt: f32, input: &InputState) -> Result<()>;
    fn render(&mut self, renderer: &mut dyn Renderer) -> Result<()>;

    fn camera(&self) -> Camera2d {
        Camera2d::default()
    }

    fn should_exit(&self) -> bool {
        false
    }
}

pub fn run<G>(title: &str, game: G) -> Result<()>
where
    G: Game + 'static,
{
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = RuntimeApp::new(title, game);
    event_loop.run_app(&mut app)?;

    if let Some(error) = app.error {
        Err(error)
    } else {
        Ok(())
    }
}

struct RuntimeApp<G> {
    title: String,
    game: G,
    // Drop the renderer before the window because the surface was created from the window handle.
    renderer: Option<WgpuRenderer>,
    window: Option<Window>,
    input: InputState,
    last_update: Instant,
    exit_after_frames: Option<u32>,
    rendered_frames: u32,
    error: Option<Error>,
}

impl<G> RuntimeApp<G> {
    fn new(title: &str, game: G) -> Self {
        Self {
            title: title.to_owned(),
            game,
            renderer: None,
            window: None,
            input: InputState::default(),
            last_update: Instant::now(),
            exit_after_frames: std::env::var("ALIEN_ARCHIVE_EXIT_AFTER_FRAMES")
                .ok()
                .and_then(|value| value.parse::<u32>().ok()),
            rendered_frames: 0,
            error: None,
        }
    }

    fn record_error(&mut self, event_loop: &ActiveEventLoop, context: &str, error: Error) {
        eprintln!("{context}: {error:?}");
        self.error = Some(error);
        event_loop.exit();
    }

    fn create_window_and_renderer(&mut self, event_loop: &ActiveEventLoop)
    where
        G: Game,
    {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(error) => {
                self.record_error(event_loop, "window creation error", Error::from(error));
                return;
            }
        };

        self.input.apply_window_resized(window.inner_size());
        let mut renderer = match pollster::block_on(WgpuRenderer::new(&window)) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.record_error(event_loop, "renderer creation error", error);
                return;
            }
        };

        if let Err(error) = self.game.setup(&mut renderer) {
            self.record_error(event_loop, "setup error", error);
            return;
        }

        self.last_update = Instant::now();
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn update_and_request_redraw(&mut self, event_loop: &ActiveEventLoop)
    where
        G: Game,
    {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32().min(0.05);
        self.last_update = now;

        if let Err(error) = self.game.update(dt, &self.input) {
            self.record_error(event_loop, "update error", error);
            return;
        }

        if self.game.should_exit() {
            event_loop.exit();
            return;
        }

        self.input.clear_transitions();
        window.request_redraw();
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop)
    where
        G: Game,
    {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        renderer.begin_frame(self.game.camera());
        let render_result = self
            .game
            .render(renderer)
            .and_then(|_| renderer.finish_frame());

        if let Err(error) = render_result {
            self.record_error(event_loop, "render error", error);
            return;
        }

        self.rendered_frames += 1;
        if self
            .exit_after_frames
            .is_some_and(|limit| self.rendered_frames >= limit)
        {
            event_loop.exit();
        }
    }
}

impl<G> ApplicationHandler for RuntimeApp<G>
where
    G: Game,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window_and_renderer(event_loop);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer = None;
        self.window = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.input.apply_window_resized(size);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.input.apply_key_event(&event),
            WindowEvent::CursorMoved { position, .. } => self.input.apply_cursor_moved(position),
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.apply_mouse_event(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.apply_mouse_wheel(delta),
            WindowEvent::RedrawRequested => self.render_frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        self.update_and_request_redraw(event_loop);
    }
}
