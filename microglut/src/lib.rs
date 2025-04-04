use glow::{Context, HasContext, NO_ERROR};
use sdl2::keyboard::{Keycode, Mod, Scancode};
use sdl2::mouse::MouseButton;

use std::marker::PhantomData;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

pub use glam;
pub use glow;
#[cfg(feature = "imgui")]
pub use imgui;
pub use sdl2::{self, video::Window};

pub mod fbo;
mod load_shaders;
mod model;
mod texture;
pub mod time;
pub mod util;

pub use fbo::FBO;
pub use load_shaders::{load_compute_shader, load_shaders, LoadShaders};
pub use model::{MaterialBindings, Model};
pub use texture::Texture;
use time::set_delta_time;
pub use time::{delta_time, elapsed_time};

pub fn print_error(gl: &Context, what: &str) -> Result<(), ()> {
    let mut result = Ok(());
    unsafe {
        let mut error = gl.get_error();
        while error != NO_ERROR {
            result = Err(());
            println!("{what}: error: {error}");
            error = gl.get_error();
        }
    }
    result
}

#[allow(unused)]
pub trait MicroGLUT: Sized {
    fn init(gl: &Context, window: &Window) -> Self;
    fn display(&mut self, gl: &Context, window: &Window);
    #[cfg(feature = "imgui")]
    fn ui(&mut self, gl: &Context, ui: &mut imgui::Ui) {}

    fn mouse_up(&mut self, button: MouseButton, x: i32, y: i32) {}
    fn mouse_down(&mut self, button: MouseButton, x: i32, y: i32) {}
    fn mouse_moved_to(&mut self, x: i32, y: i32) {}
    fn mouse_moved_rel(&mut self, xrel: i32, yrel: i32) {}

    fn key_down(
        &mut self,
        keycode: Option<Keycode>,
        scancode: Option<Scancode>,
        keymod: Mod,
        repeat: bool,
    ) {
    }

    fn key_up(
        &mut self,
        keycode: Option<Keycode>,
        scancode: Option<Scancode>,
        keymod: Mod,
        repeat: bool,
    ) {
    }

    fn sdl2_window(window_title: impl Into<String>) -> StartBuilder<Self> {
        StartBuilder::new(window_title.into())
    }
}

pub type DebugMessageCallback = dyn Fn(u32, u32, u32, u32, String) + Send + Sync;

pub struct StartBuilder<T: MicroGLUT> {
    window_title: String,
    window_width: Option<u32>,
    window_height: Option<u32>,
    gl_version: Option<(u8, u8)>,
    micro_glut: PhantomData<T>,
    debug_message_callback: Option<Box<DebugMessageCallback>>,
    imgui_ini_filename: Option<String>,
}

impl<T: MicroGLUT> StartBuilder<T> {
    pub fn new(window_title: String) -> Self {
        StartBuilder {
            window_title,
            window_width: None,
            window_height: None,
            gl_version: None,
            micro_glut: PhantomData,
            debug_message_callback: None,
            imgui_ini_filename: None,
        }
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = Some(width);
        self.window_height = Some(height);
        self
    }

    pub fn gl_version(mut self, major: u8, minor: u8) -> Self {
        self.gl_version = Some((major, minor));
        self
    }

    pub fn debug_message_callback(
        mut self,
        callback: impl Fn(u32, u32, u32, u32, String) + Send + Sync + 'static,
    ) -> Self {
        self.debug_message_callback = Some(Box::new(callback));
        self
    }

    pub fn imgui_ini_filename(mut self, filename: impl Into<String>) -> Self {
        self.imgui_ini_filename = Some(filename.into());
        self
    }

    pub fn start(mut self) {
        time::initialize();

        let (gl_major_version, gl_minor_version) = self.gl_version.unwrap_or((3, 2));

        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(gl_major_version, gl_minor_version);
        let window = video
            .window(
                &self.window_title,
                self.window_width.unwrap_or(800),
                self.window_height.unwrap_or(800),
            )
            .allow_highdpi()
            .opengl()
            .resizable()
            .build()
            .unwrap();
        let gl_context = window.gl_create_context().unwrap();
        window.gl_make_current(&gl_context).unwrap();
        window.subsystem().gl_set_swap_interval(1).unwrap();

        let mut gl =
            unsafe { Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _) };

        if let Some(callback) = self.debug_message_callback.take() {
            unsafe {
                gl.debug_message_callback(move |source, typ, id, severity, message| {
                    callback(source, typ, id, severity, message.to_string())
                });
            }
        }

        let mut app = T::init(&gl, &window);
        let mut prev_frame = Instant::now();

        #[cfg(feature = "imgui")]
        let (mut imgui, mut platform, mut renderer, gl) = {
            let mut imgui = imgui::Context::create();
            imgui.set_ini_filename(self.imgui_ini_filename.map(PathBuf::from));
            imgui.set_log_filename(None);

            imgui
                .fonts()
                .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

            let platform = imgui_sdl2_support::SdlPlatform::new(&mut imgui);
            let renderer = imgui_glow_renderer::AutoRenderer::new(gl, &mut imgui).unwrap();
            let gl = Rc::clone(renderer.gl_context());
            (imgui, platform, renderer, gl)
        };
        #[cfg(not(feature = "imgui"))]
        let gl = Rc::new(gl);

        let mut running = true;
        let mut event_loop = sdl.event_pump().unwrap();
        while running {
            for event in event_loop.poll_iter() {
                use sdl2::event::Event;

                #[cfg(feature = "imgui")]
                platform.handle_event(&mut imgui, &event);

                match event {
                    Event::Quit { .. } => running = false,
                    Event::MouseButtonUp {
                        mouse_btn, x, y, ..
                    } => app.mouse_up(mouse_btn, x, y),
                    Event::MouseButtonDown {
                        mouse_btn, x, y, ..
                    } => app.mouse_down(mouse_btn, x, y),
                    Event::MouseMotion {
                        x, y, xrel, yrel, ..
                    } => {
                        app.mouse_moved_to(x, y);
                        app.mouse_moved_rel(xrel, yrel);
                    }
                    Event::KeyDown {
                        keycode,
                        scancode,
                        keymod,
                        repeat,
                        ..
                    } => {
                        app.key_down(keycode, scancode, keymod, repeat);
                    }
                    Event::KeyUp {
                        keycode,
                        scancode,
                        keymod,
                        repeat,
                        ..
                    } => app.key_up(keycode, scancode, keymod, repeat),
                    _ => {}
                }
            }

            #[cfg(feature = "imgui")]
            {
                platform.prepare_frame(&mut imgui, &window, &event_loop);
                let ui = imgui.frame();

                app.ui(&gl, ui);
            }

            app.display(&gl, &window);

            #[cfg(feature = "imgui")]
            {
                let draw_data = imgui.render();
                renderer.render(draw_data).unwrap();
            }

            window.gl_swap_window();

            let now = Instant::now();
            set_delta_time(now.duration_since(prev_frame).as_secs_f32());
            prev_frame = now;
        }
    }
}
