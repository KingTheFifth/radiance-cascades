use std::f32::consts::PI;

use glam::{Mat4, Vec2, Vec3};
use glow::{
    Context, HasContext, NativeProgram, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT,
    CULL_FACE, DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, LINEAR, LINEAR_MIPMAP_LINEAR, STATIC_DRAW,
    TEXTURE_2D, TEXTURE_MIN_FILTER, TRIANGLES,
};
use microglut::{delta_time, load_shaders, MicroGLUT, Texture};
use sdl2::{
    keyboard::{Keycode, Mod, Scancode},
    video::Window,
};

struct Demo {
    program: NativeProgram,
    vao: NativeVertexArray,
    min_filter: u32,
    animate: bool,
    t: f32,
}

impl MicroGLUT for Demo {
    fn init(gl: &Context, _window: &Window) -> Self {
        let vertex_data = [
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(-0.5, 0.5, -0.5),  // 3
            Vec3::new(0.5, 0.5, -0.5),   // 2
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(0.5, 0.5, -0.5),   // 2
            Vec3::new(0.5, -0.5, -0.5),  // 1
            //
            Vec3::new(0.5, 0.5, -0.5),  // 2
            Vec3::new(-0.5, 0.5, -0.5), // 3
            Vec3::new(-0.5, 0.5, 0.5),  // 7
            Vec3::new(0.5, 0.5, -0.5),  // 2
            Vec3::new(-0.5, 0.5, 0.5),  // 7
            Vec3::new(0.5, 0.5, 0.5),   // 6
            //
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(-0.5, -0.5, 0.5),  // 4
            Vec3::new(-0.5, 0.5, 0.5),   // 7
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(-0.5, 0.5, 0.5),   // 7
            Vec3::new(-0.5, 0.5, -0.5),  // 3
            //
            Vec3::new(0.5, -0.5, -0.5), // 1
            Vec3::new(0.5, 0.5, -0.5),  // 2
            Vec3::new(0.5, 0.5, 0.5),   // 6
            Vec3::new(0.5, -0.5, -0.5), // 1
            Vec3::new(0.5, 0.5, 0.5),   // 6
            Vec3::new(0.5, -0.5, 0.5),  // 5
            //
            Vec3::new(-0.5, -0.5, 0.5), // 4
            Vec3::new(0.5, -0.5, 0.5),  // 5
            Vec3::new(0.5, 0.5, 0.5),   // 6
            Vec3::new(-0.5, -0.5, 0.5), // 4
            Vec3::new(0.5, 0.5, 0.5),   // 6
            Vec3::new(-0.5, 0.5, 0.5),  // 7
            //
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(0.5, -0.5, -0.5),  // 1
            Vec3::new(0.5, -0.5, 0.5),   // 5
            Vec3::new(-0.5, -0.5, -0.5), // 0
            Vec3::new(0.5, -0.5, 0.5),   // 5
            Vec3::new(-0.5, -0.5, 0.5),  // 4
        ];

        let texcoord_data = [
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            //
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            //
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            //
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            //
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            //
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ];

        let projection_matrix = Mat4::perspective_rh(PI / 2.0, 1.0, 1.0, 30.0);

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            gl.disable(CULL_FACE);

            let program = load_shaders(
                gl,
                include_str!("texcube.vert"),
                include_str!("texcube.frag"),
            );
            gl.use_program(Some(program));

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vertex_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vertex_vbo));
            gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                bytemuck::cast_slice(&vertex_data),
                STATIC_DRAW,
            );
            let loc = gl.get_attrib_location(program, "in_Position").unwrap();
            gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(loc);

            let texcoord_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(texcoord_vbo));
            gl.buffer_data_u8_slice(
                ARRAY_BUFFER,
                bytemuck::cast_slice(&texcoord_data),
                STATIC_DRAW,
            );
            let loc = gl.get_attrib_location(program, "in_Texcoord").unwrap();
            gl.vertex_attrib_pointer_f32(loc, 2, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(loc);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(program, "projMatrix").as_ref(),
                false,
                projection_matrix.as_ref(),
            );

            gl.uniform_1_i32(gl.get_uniform_location(program, "tex").as_ref(), 0);
            let _maskros = Texture::load(gl, include_bytes!("maskros512.tga"), true);

            Demo {
                program,
                vao,
                animate: true,
                min_filter: LINEAR_MIPMAP_LINEAR,
                t: 0.0,
            }
        }
    }

    fn display(&mut self, gl: &Context, _window: &Window) {
        if self.animate {
            self.t += delta_time();
        }
        let t = self.t;
        let m = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0))
            * Mat4::from_rotation_z(PI / 2.0)
            * Mat4::from_rotation_y(t)
            * Mat4::from_scale(Vec3::new(
                (t / 2.0).sin() / 4.0 + 0.75,
                (t / 2.0).sin() / 4.0 + 0.75,
                (t / 2.0).sin() / 4.0 + 0.75,
            ));

        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, self.min_filter as _);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "modelviewMatrix")
                    .as_ref(),
                false,
                m.as_ref(),
            );

            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(TRIANGLES, 0, 36 * 3);
        }
    }

    #[cfg(feature = "imgui")]
    fn ui(&mut self, _gl: &Context, ui: &mut imgui::Ui) {
        ui.window("debug").build(|| {
            if ui.radio_button_bool("LINEAR", self.min_filter == LINEAR) {
                self.min_filter = LINEAR;
            }
            if ui.radio_button_bool(
                "LINEAR_MIPMAP_LINEAR",
                self.min_filter == LINEAR_MIPMAP_LINEAR,
            ) {
                self.min_filter = LINEAR_MIPMAP_LINEAR;
            }
            ui.separator();
            ui.checkbox("animate", &mut self.animate);
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
            Keycode::Plus => self.min_filter = LINEAR_MIPMAP_LINEAR,
            Keycode::Minus => self.min_filter = LINEAR,
            Keycode::Space => self.animate ^= true,
            _ => {}
        }
    }
}

fn main() {
    Demo::sdl2_window("Color cube example").start();
}
