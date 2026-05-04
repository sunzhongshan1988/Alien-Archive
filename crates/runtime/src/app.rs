use std::time::Instant;

use anyhow::Result;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
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

pub fn run<G>(title: &str, mut game: G) -> Result<()>
where
    G: Game + 'static,
{
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .with_min_inner_size(LogicalSize::new(640.0, 360.0))
        .build(&event_loop)?;

    let mut input = InputState::default();
    input.apply_window_resized(window.inner_size());

    let mut renderer = pollster::block_on(WgpuRenderer::new(&window))?;
    game.setup(&mut renderer)?;

    let mut last_update = Instant::now();
    let exit_after_frames = std::env::var("ALIEN_ARCHIVE_EXIT_AFTER_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let mut rendered_frames = 0_u32;

    event_loop.run(move |event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    input.apply_window_resized(size);
                    renderer.resize(size);
                }
                WindowEvent::KeyboardInput { event, .. } => input.apply_key_event(&event),
                WindowEvent::CursorMoved { position, .. } => input.apply_cursor_moved(position),
                WindowEvent::MouseInput { state, button, .. } => {
                    input.apply_mouse_event(button, state);
                }
                WindowEvent::RedrawRequested => {
                    renderer.begin_frame(game.camera());
                    let render_result = game
                        .render(&mut renderer)
                        .and_then(|_| renderer.finish_frame());

                    if let Err(error) = render_result {
                        eprintln!("render error: {error:?}");
                        event_loop.exit();
                    }

                    rendered_frames += 1;
                    if exit_after_frames.is_some_and(|limit| rendered_frames >= limit) {
                        event_loop.exit();
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                let now = Instant::now();
                let dt = (now - last_update).as_secs_f32().min(0.05);
                last_update = now;

                if let Err(error) = game.update(dt, &input) {
                    eprintln!("update error: {error:?}");
                    event_loop.exit();
                    return;
                }

                if game.should_exit() {
                    event_loop.exit();
                    return;
                }

                input.clear_transitions();
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}
