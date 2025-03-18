use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use glow::{
    Context, HasContext, NativeProgram, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT,
    CULL_FACE, DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, STATIC_DRAW, TRIANGLES,
};
use microglut::{elapsed_time, load_shaders, MicroGLUT};
use sdl2::video::Window;

struct Demo {
    program: NativeProgram,
    vao: NativeVertexArray,
    rotation_matrix: Mat4,
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

        let red = Vec3::X;
        let green = Vec3::Y;
        let blue = Vec3::Z;
        let cyan = green + blue;
        let magenta = red + blue;
        let yellow = red + green;
        let color_data = [red, green, blue, cyan, magenta, yellow]
            .into_iter()
            .flat_map(|c| std::iter::repeat(c).take(6))
            .collect::<Vec<_>>();

        let translation_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0));
        let projection_matrix = Mat4::perspective_rh(PI / 2.0, 1.0, 1.0, 30.0);

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            gl.disable(CULL_FACE);

            let program = load_shaders(
                gl,
                include_str!("colorcube.vert"),
                include_str!("colorcube.frag"),
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

            let color_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(color_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&color_data), STATIC_DRAW);
            let loc = gl.get_attrib_location(program, "in_Color").unwrap();
            gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(loc);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(program, "translationMatrix")
                    .as_ref(),
                false,
                translation_matrix.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(program, "projMatrix").as_ref(),
                false,
                projection_matrix.as_ref(),
            );

            Demo {
                program,
                vao,
                rotation_matrix: Mat4::IDENTITY,
            }
        }
    }

    fn display(&mut self, gl: &Context, _window: &Window) {
        let t = elapsed_time();
        let m = self.rotation_matrix.as_mut();

        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            // NOTE: Transposed matrix!
            m[0] = (t / 5.0).cos();
            m[1] = -(t / 5.0).sin();
            m[4] = (t / 5.0).sin();
            m[5] = (t / 5.0).cos();
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "rotationMatrix2")
                    .as_ref(),
                true,
                m.as_ref(),
            );
            m[5] = t.cos();
            m[6] = -t.sin();
            m[9] = t.sin();
            m[10] = t.cos();
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "rotationMatrix")
                    .as_ref(),
                true,
                m.as_ref(),
            );

            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(TRIANGLES, 0, 36 * 3);
        }
    }
}

fn main() {
    Demo::sdl2_window("Color cube example").start();
}
