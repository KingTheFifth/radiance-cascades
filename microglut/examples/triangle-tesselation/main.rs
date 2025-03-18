use glow::{
    HasContext as _, Program, VertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT, DEPTH_TEST, FLOAT,
    FRONT_AND_BACK, LINE, PATCHES, STATIC_DRAW,
};
use microglut::{print_error, LoadShaders, MicroGLUT};
use sdl2::keyboard::{Keycode, Mod, Scancode};

struct Demo {
    program: Program,
    vao: VertexArray,
    tess_level_inner: u32,
    tess_level_outer: u32,
}

impl MicroGLUT for Demo {
    fn init(gl: &glow::Context, _window: &sdl2::video::Window) -> Self {
        #[rustfmt::skip]
        let vertices = [
            -0.5, -0.5, 0.0,
            -0.5,  0.5, 0.0,
             0.5, -0.5, 0.0f32,
        ];

        let program = LoadShaders::new(include_str!("minimal.vs"), include_str!("minimal.fs"))
            .tesselation(include_str!("minimal.tcs"), include_str!("minimal.tes"))
            .compile(gl);

        print_error(gl, "init (load shaders)").unwrap();

        unsafe {
            gl.use_program(Some(program));

            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.disable(DEPTH_TEST);

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));
            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(
                gl.get_attrib_location(program, "in_Position").unwrap(),
                3,
                FLOAT,
                false,
                0,
                0,
            );
            gl.enable_vertex_attrib_array(gl.get_attrib_location(program, "in_Position").unwrap());

            print_error(gl, "init (upload vertex data)").unwrap();

            // Wireframe please!
            gl.polygon_mode(FRONT_AND_BACK, LINE);

            print_error(gl, "init (polygon mode)").unwrap();

            Demo {
                program,
                vao,
                tess_level_inner: 2,
                tess_level_outer: 2,
            }
        }
    }

    fn display(&mut self, gl: &glow::Context, _window: &sdl2::video::Window) {
        unsafe {
            gl.clear(COLOR_BUFFER_BIT);
            print_error(gl, "display (clear)").unwrap();

            gl.bind_vertex_array(Some(self.vao));
            print_error(gl, "display (bind vertex array)").unwrap();

            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelInner")
                    .as_ref(),
                self.tess_level_inner as _,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelOuter")
                    .as_ref(),
                self.tess_level_outer as _,
            );
            print_error(gl, "init (uniforms tess level {inner,outer})").unwrap();

            // gl.patch_parameter_i32(PATCH_DEFAULT_OUTER_LEVEL, 3);
            // gl.patch_parameter_i32(PATCH_DEFAULT_INNER_LEVEL, 3);
            // gl.patch_parameter_i32(PATCH_VERTICES, 3);
            // print_error(gl, "display (patch parameter)").unwrap();
            gl.draw_arrays(PATCHES, 0, 3);
            print_error(gl, "display (draw arrays, patches)").unwrap();
            // gl.draw_arrays(TRIANGLES, 0, 3);
            // print_error(gl, "display (draw arrays, triangles)").unwrap();
        }
    }

    #[cfg(feature = "imgui")]
    fn ui(&mut self, _gl: &glow::Context, ui: &mut imgui::Ui) {
        ui.window("debug")
            .size([400., -1.], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.slider("Tesselation level inner", 0, 20, &mut self.tess_level_inner);
                ui.slider("Tesselation level outer", 0, 20, &mut self.tess_level_outer);
            });
    }

    fn key_down(
        &mut self,
        keycode: Option<Keycode>,
        _scancode: Option<Scancode>,
        _keymod: Mod,
        _repeat: bool,
    ) {
        let changed = match keycode {
            Some(Keycode::Plus) => {
                self.tess_level_inner += 1;
                true
            }
            Some(Keycode::Minus) => {
                self.tess_level_inner = self.tess_level_inner.saturating_sub(1);
                true
            }
            Some(Keycode::Period) => {
                self.tess_level_outer += 1;
                true
            }
            Some(Keycode::Comma) => {
                self.tess_level_outer = self.tess_level_outer.saturating_sub(1);
                true
            }
            _ => false,
        };

        if changed {
            println!(
                "tess_level_inner={}, tess_level_outer={}",
                self.tess_level_inner, self.tess_level_outer
            );
        }
    }
}

fn main() {
    Demo::sdl2_window("Triangle tesselation example")
        .window_size(800, 800)
        .gl_version(4, 2)
        .start();
}
