extern crate gl;
extern crate image;
pub extern crate nalgebra_glm as glm;
extern crate sdl2;

pub mod error;
pub mod graphics;
pub mod input;
pub mod time;

use std::time::{Duration, Instant};

use glm::Vec2;
use sdl2::event::Event;
pub use sdl2::keyboard::Keycode as Key;
use sdl2::video::Window;
use sdl2::Sdl;

use error::{Result, TetraError};
use graphics::opengl::GLDevice;
use graphics::GraphicsContext;
use input::InputContext;

pub trait State {
    fn update(&mut self, ctx: &mut Context);
    fn draw(&mut self, ctx: &mut Context, dt: f64);
}

pub struct Context {
    sdl: Sdl,
    window: Window,
    gl: GLDevice,
    graphics: GraphicsContext,
    input: InputContext,

    running: bool,
    quit_on_escape: bool,
    tick_rate: f64,
}

pub struct ContextBuilder<'a> {
    title: &'a str,
    width: u32,
    height: u32,
    scale: u32,
    vsync: bool,
    quit_on_escape: bool,
}

impl<'a> ContextBuilder<'a> {
    pub fn new() -> ContextBuilder<'a> {
        ContextBuilder {
            title: "Tetra",
            width: 1280,
            height: 720,
            scale: 1,
            vsync: true,
            quit_on_escape: false,
        }
    }

    pub fn title(mut self, title: &'a str) -> ContextBuilder<'a> {
        self.title = title;
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> ContextBuilder<'a> {
        self.width = width;
        self.height = height;
        self
    }

    pub fn scale(mut self, scale: u32) -> ContextBuilder<'a> {
        self.scale = scale;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> ContextBuilder<'a> {
        self.vsync = vsync;
        self
    }

    pub fn quit_on_escape(mut self, quit_on_escape: bool) -> ContextBuilder<'a> {
        self.quit_on_escape = quit_on_escape;
        self
    }

    pub fn build(self) -> Result<Context> {
        let sdl = sdl2::init().map_err(TetraError::Sdl)?;
        let video = sdl.video().map_err(TetraError::Sdl)?;

        let window = video
            .window(
                self.title,
                self.width * self.scale,
                self.height * self.scale,
            ).position_centered()
            .opengl()
            .build()
            .map_err(|e| TetraError::Sdl(e.to_string()))?; // TODO: This could probably be cleaner

        let mut gl = GLDevice::new(&video, &window, self.vsync)?;
        let graphics = GraphicsContext::new(
            &mut gl,
            self.width as i32,
            self.height as i32,
            self.scale as i32,
        );
        let input = InputContext::new();

        Ok(Context {
            sdl,
            window,
            gl,
            graphics,
            input,

            running: false,
            quit_on_escape: self.quit_on_escape,
            tick_rate: 1.0 / 60.0,
        })
    }
}

pub fn run<T: State>(ctx: &mut Context, state: &mut T) -> Result {
    let mut events = ctx.sdl.event_pump().map_err(TetraError::Sdl)?;

    let mut last_time = Instant::now();
    let mut lag = Duration::from_secs(0);
    let tick_rate = time::f64_to_duration(ctx.tick_rate);

    ctx.running = true;

    while ctx.running {
        let current_time = Instant::now();
        let elapsed = current_time - last_time;
        last_time = current_time;
        lag += elapsed;

        ctx.input.previous_key_state = ctx.input.current_key_state;

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => ctx.running = false, // TODO: Add a way to override this
                Event::KeyDown {
                    keycode: Some(k), ..
                } => {
                    if let Key::Escape = k {
                        if ctx.quit_on_escape {
                            ctx.running = false;
                        }
                    }

                    ctx.input.current_key_state[k as usize] = true;
                }
                Event::KeyUp {
                    keycode: Some(k), ..
                } => {
                    // TODO: This can cause some inputs to be missed at low tick rates.
                    // Could consider buffering input releases like Otter2D does?
                    ctx.input.current_key_state[k as usize] = false;
                }
                Event::MouseMotion { x, y, .. } => {
                    ctx.input.mouse_position = Vec2::new(x as f32, y as f32)
                }
                _ => {}
            }
        }

        while lag >= tick_rate {
            state.update(ctx);
            lag -= tick_rate;
        }

        let dt = time::duration_to_f64(lag) / ctx.tick_rate;

        state.draw(ctx, dt);

        graphics::present(ctx);

        std::thread::yield_now();
    }

    Ok(())
}

pub fn quit(ctx: &mut Context) {
    ctx.running = false;
}
