use glam::{Mat4, Vec3};
use glow::{
    Context, HasContext as _, NativeProgram, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT,
    DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, STATIC_DRAW, TRIANGLES,
};
use microglut::{elapsed_time, load_shaders, print_error, MicroGLUT};
use sdl2::video::Window;

struct Demo {
    vao: NativeVertexArray,
    program: NativeProgram,
}

impl MicroGLUT for Demo {
    fn init(gl: &Context, _window: &Window) -> Self {
        let vertices = [
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
        ];

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);

            let program = load_shaders(
                gl,
                include_str!("rotationanimation.vert"),
                include_str!("rotationanimation.frag"),
            );
            gl.use_program(Some(program));

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);
            let position_loc = gl.get_attrib_location(program, "in_Position").unwrap();
            gl.vertex_attrib_pointer_f32(position_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(position_loc);

            print_error(gl, "init").unwrap();

            Demo { vao, program }
        }
    }

    fn display(&mut self, gl: &Context, _window: &Window) {
        let t = elapsed_time();
        let rotation_matrix = Mat4::from_rotation_z(t * 2.0)
            * Mat4::from_translation(Vec3::new(t.sin() / 2.0, 0.0, 0.0));

        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "myMatrix").as_ref(),
                false,
                rotation_matrix.as_ref(),
            );

            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(TRIANGLES, 0, 3);
            print_error(gl, "display").unwrap();
        }
    }
}

fn main() {
    Demo::sdl2_window("Rotation animation example").start();
}
