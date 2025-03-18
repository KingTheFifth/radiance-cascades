use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use glow::{
    Context, HasContext as _, Program, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, DEPTH_TEST, PATCHES,
    PATCH_VERTICES, TEXTURE0, UNSIGNED_INT,
};
use microglut::{print_error, LoadShaders, MicroGLUT, Model, Texture};
use sdl2::{
    keyboard::{Keycode, Mod, Scancode},
    video::Window,
};

struct Demo {
    program: Program,
    tess_level_inner: u32,
    tess_level_outer: [u32; 3],
    teapot: Model,
    texon: u32,
    disp: f32,
    anim: bool,
}

impl MicroGLUT for Demo {
    fn init(gl: &Context, _window: &Window) -> Self {
        let teapot = Model::load_obj_data(gl, include_bytes!("teapotx.obj"), None, None);

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            let program = LoadShaders::new(include_str!("minimal.vs"), include_str!("minimal.fs"))
                .geometry(include_str!("minimal.gs"))
                .tesselation(include_str!("minimal.tcs"), include_str!("minimal.tes"))
                .compile(gl);
            gl.use_program(Some(program));

            gl.active_texture(TEXTURE0);
            let _ = Texture::load(gl, include_bytes!("spots.tga"), false);
            gl.uniform_1_i32(gl.get_uniform_location(program, "tex").as_ref(), 0);
            // gl.polygon_mode(FRONT_AND_BACK, LINE);

            print_error(gl, "init").unwrap();

            Demo {
                program,
                tess_level_inner: 2,
                tess_level_outer: [2, 2, 2],
                teapot,
                texon: 0,
                disp: 0.0,
                anim: false,
            }
        }
    }

    fn display(&mut self, gl: &Context, _window: &Window) {
        let projection_matrix = Mat4::perspective_rh(PI / 2.0, 1.0, 0.1, 300.0);
        let world_to_view = Mat4::look_at_rh(Vec3::new(0.0, 1.0, 3.0), Vec3::ZERO, Vec3::Y);
        let model_to_world = Mat4::from_rotation_y(0.0);

        // TODO: animate

        let m = projection_matrix * world_to_view * model_to_world;

        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            // gl.patch_parameter_i32(PATCH_DEFAULT_OUTER_LEVEL, 3);
            // gl.patch_parameter_i32(PATCH_DEFAULT_INNER_LEVEL, 3);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelInner")
                    .as_ref(),
                self.tess_level_inner as _,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelOuter1")
                    .as_ref(),
                self.tess_level_outer[0] as _,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelOuter2")
                    .as_ref(),
                self.tess_level_outer[1] as _,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "TessLevelOuter3")
                    .as_ref(),
                self.tess_level_outer[2] as _,
            );
            print_error(gl, "display (patch parameter 1)").unwrap();
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "disp").as_ref(),
                self.disp,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "texon").as_ref(),
                self.texon as _,
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "m").as_ref(),
                false,
                m.as_ref(),
            );
            print_error(gl, "display (uniform)").unwrap();

            gl.patch_parameter_i32(PATCH_VERTICES, 3);
            print_error(gl, "display (patch parameter 2)").unwrap();
            self.teapot.meshes[0].bind(
                gl,
                self.program,
                "in_Position",
                Some("in_Normal"),
                Some("in_TexCoord"),
            );
            gl.draw_elements(
                PATCHES,
                self.teapot.meshes[0].num_indices() as _,
                UNSIGNED_INT,
                0,
            );
            print_error(gl, "display (draw elements)").unwrap();

            print_error(gl, "display").unwrap();
        }
    }

    #[cfg(feature = "imgui")]
    fn ui(&mut self, _gl: &Context, ui: &mut imgui::Ui) {
        ui.window("debug")
            .size([400., -1.], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.slider("Displacement", 0.0, 2.0, &mut self.disp);
                ui.slider("Inner tesselation", 00, 20, &mut self.tess_level_inner);
                ui.slider(
                    "Outer tesselation (1)",
                    0,
                    20,
                    &mut self.tess_level_outer[0],
                );
                ui.slider(
                    "Outer tesselation (2)",
                    00,
                    20,
                    &mut self.tess_level_outer[1],
                );
                ui.slider(
                    "Outer tesselation (3)",
                    00,
                    20,
                    &mut self.tess_level_outer[2],
                );
            });
    }

    fn key_down(
        &mut self,
        keycode: Option<Keycode>,
        _scancode: Option<Scancode>,
        _keymod: Mod,
        _repeat: bool,
    ) {
        let Some(keycode) = keycode else {
            return;
        };
        match keycode {
            Keycode::T => self.texon = (self.texon + 1) % 3,
            Keycode::A => self.disp += 0.05,
            Keycode::Z => self.disp -= 0.05,
            Keycode::Plus => self.tess_level_inner += 1,
            Keycode::Minus => self.tess_level_inner = self.tess_level_inner.saturating_sub(1),
            Keycode::Period => {
                self.tess_level_outer[0] += 1;
                self.tess_level_outer[1] += 1;
                self.tess_level_outer[2] += 1;
            }
            Keycode::Comma => {
                self.tess_level_outer[0] = self.tess_level_outer[0].saturating_sub(1);
                self.tess_level_outer[1] = self.tess_level_outer[1].saturating_sub(1);
                self.tess_level_outer[2] = self.tess_level_outer[2].saturating_sub(1);
            }
            Keycode::Num1 => self.tess_level_outer[0] = self.tess_level_outer[0].saturating_sub(1),
            Keycode::Num2 => self.tess_level_outer[0] += 1,
            Keycode::Num3 => self.tess_level_outer[1] = self.tess_level_outer[1].saturating_sub(1),
            Keycode::Num4 => self.tess_level_outer[1] += 1,
            Keycode::Num5 => self.tess_level_outer[2] = self.tess_level_outer[2].saturating_sub(1),
            Keycode::Num6 => self.tess_level_outer[2] += 1,
            Keycode::Space => self.anim = !self.anim,
            _ => (),
        }
    }
}

fn main() {
    Demo::sdl2_window("heavy metal teapot")
        .window_size(800, 800)
        .gl_version(4, 1)
        .start();
}
