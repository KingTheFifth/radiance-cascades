use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use glow::{
    HasContext, Program, BACK, BLEND, COLOR_BUFFER_BIT, CULL_FACE, DEPTH_BUFFER_BIT, DEPTH_TEST,
    FRONT, LINEAR, LINEAR_MIPMAP_LINEAR, NEAREST, ONE_MINUS_SRC_ALPHA, REPEAT, RGBA, SRC_ALPHA,
    TEXTURE0, TEXTURE_2D, TEXTURE_CUBE_MAP, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_S,
    TEXTURE_WRAP_T, UNSIGNED_BYTE,
};
use microglut::{load_shaders, print_error, util::arb_rotate, MicroGLUT, Model};
use sdl2::keyboard::{Keycode, Mod, Scancode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransparentSetting {
    NoCullNoZ,
    NoCullZ,
    CullBackZ,
    CullBothZ,
}

struct TransparentTeapot {
    program: Program,
    teapot: Model,
    angle: f32,
    setting: TransparentSetting,
}

impl MicroGLUT for TransparentTeapot {
    fn init(gl: &glow::Context, _window: &sdl2::video::Window) -> Self {
        #[rustfmt::skip]
        let minitexrgba = [
            255, 0,   255, 255, /**/  0,   0, 255, 128, /**/ 0,   0  , 255, 128, /**/ 0,   255, 255, 255,
            0,   0,   255, 128, /**/  255, 0, 255, 0,   /**/ 0,   255, 255, 0,   /**/ 0,   0,   255, 128,
            0,   0,   255, 128, /**/  0, 255, 255, 0,   /**/ 255, 0,   255, 0,   /**/ 0,   0,   255, 128,
            0,   255, 255, 255, /**/  0,   0, 255, 128, /**/ 0,   0,   255, 128, /**/ 255, 0,   255, 255,
        ];

        unsafe {
            let program = load_shaders(gl, include_str!("tex.vert"), include_str!("tex.frag"));
            let teapot = Model::load_obj_data(gl, include_bytes!("teapotmini.obj"), None, None);
            gl.use_program(Some(program));

            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(BLEND);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            print_error(gl, "init (blend)").unwrap();

            let tex = gl.create_texture().unwrap();
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_CUBE_MAP, Some(tex));
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as _,
                4,
                4,
                0,
                RGBA,
                UNSIGNED_BYTE,
                Some(&minitexrgba),
            );
            print_error(gl, "init (tex image)").unwrap();

            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT as _);
            print_error(gl, "init (tex parameter wrap)").unwrap();
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
            print_error(gl, "init (tex parameter minmag filter)").unwrap();

            gl.tex_parameter_i32(
                TEXTURE_CUBE_MAP,
                TEXTURE_MIN_FILTER,
                LINEAR_MIPMAP_LINEAR as _,
            );
            print_error(gl, "init (tex parameter cube map filter)").unwrap();
            // gl.generate_mipmap(TEXTURE_CUBE_MAP);
            print_error(gl, "init (tex mipmap)").unwrap();

            gl.uniform_1_i32(gl.get_uniform_location(program, "tex").as_ref(), 0);

            print_error(gl, "init").unwrap();

            TransparentTeapot {
                program,
                teapot,
                angle: 0.0,
                setting: TransparentSetting::NoCullNoZ,
            }
        }
    }

    fn display(&mut self, gl: &glow::Context, _window: &sdl2::video::Window) {
        let projection = Mat4::perspective_rh(PI / 2.0, 1.0, 0.2, 100.0);
        let look_at = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO, Vec3::Y);

        let dt = microglut::delta_time();
        self.angle += dt;

        let rotation = arb_rotate(Vec3::new(1.0, 1.0, 1.0), self.angle);

        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "projMatrix").as_ref(),
                false,
                projection.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "lookAtMatrix")
                    .as_ref(),
                false,
                look_at.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "rotationMatrix")
                    .as_ref(),
                false,
                rotation.as_ref(),
            );

            gl.enable(DEPTH_TEST);
            gl.enable(CULL_FACE);

            match self.setting {
                TransparentSetting::NoCullNoZ => {
                    gl.disable(DEPTH_TEST);
                    gl.disable(CULL_FACE);
                    self.teapot.draw(
                        gl,
                        self.program,
                        "inPosition",
                        Some("inNormal"),
                        Some("inTexCoord"),
                    );
                }
                TransparentSetting::NoCullZ => {
                    gl.disable(CULL_FACE);
                    self.teapot.draw(
                        gl,
                        self.program,
                        "inPosition",
                        Some("inNormal"),
                        Some("inTexCoord"),
                    );
                }
                TransparentSetting::CullBackZ => {
                    gl.cull_face(BACK);
                    self.teapot.draw(
                        gl,
                        self.program,
                        "inPosition",
                        Some("inNormal"),
                        Some("inTexCoord"),
                    );
                }
                TransparentSetting::CullBothZ => {
                    gl.cull_face(FRONT);
                    self.teapot.draw(
                        gl,
                        self.program,
                        "inPosition",
                        Some("inNormal"),
                        Some("inTexCoord"),
                    );
                    gl.cull_face(BACK);
                    self.teapot.draw(
                        gl,
                        self.program,
                        "inPosition",
                        Some("inNormal"),
                        Some("inTexCoord"),
                    );
                }
            }
            print_error(gl, "display").unwrap();
        }
    }

    #[cfg(feature = "imgui")]
    fn ui(&mut self, _gl: &glow::Context, ui: &mut imgui::Ui) {
        ui.window("debug").build(|| {
            for (label, button_value) in [
                ("No culling, no depth test", TransparentSetting::NoCullNoZ),
                ("No culling, with depth test", TransparentSetting::NoCullZ),
                ("Cull back, with depth test", TransparentSetting::CullBackZ),
                ("Cull both, with depth test", TransparentSetting::CullBothZ),
            ] {
                ui.radio_button(label, &mut self.setting, button_value);
            }
        });
    }

    fn key_down(
        &mut self,
        keycode: Option<Keycode>,
        _scancode: Option<Scancode>,
        _keymod: Mod,
        _repeat: bool,
    ) {
        match keycode {
            Some(Keycode::Num1) => self.setting = TransparentSetting::NoCullNoZ,
            Some(Keycode::Num2) => self.setting = TransparentSetting::NoCullZ,
            Some(Keycode::Num3) => self.setting = TransparentSetting::CullBackZ,
            Some(Keycode::Num4) => self.setting = TransparentSetting::CullBothZ,
            _ => {}
        }
    }
}

fn main() {
    TransparentTeapot::sdl2_window("Transparent teapot")
        .window_size(500, 500)
        .start();
}
