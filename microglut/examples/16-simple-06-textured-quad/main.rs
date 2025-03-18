use glam::{Vec2, Vec3};
use glow::{
    HasContext, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, DEPTH_TEST,
    FLOAT, STATIC_DRAW, TRIANGLES,
};
use microglut::{load_shaders, MicroGLUT, Texture};

struct Demo {
    vao: NativeVertexArray,
}

impl MicroGLUT for Demo {
    fn init(gl: &glow::Context, _window: &sdl2::video::Window) -> Self {
        let vertices = [
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
        ];
        let texcoord = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ];

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            let program = load_shaders(
                gl,
                include_str!("textured.vert"),
                include_str!("textured.frag"),
            );
            gl.use_program(Some(program));

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vert_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vert_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let pos_loc = gl.get_attrib_location(program, "inPosition").unwrap();
            gl.vertex_attrib_pointer_f32(pos_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(pos_loc);

            let texcoord_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(texcoord_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&texcoord), STATIC_DRAW);

            let texcoord_loc = gl.get_attrib_location(program, "inTexCoord").unwrap();
            gl.vertex_attrib_pointer_f32(texcoord_loc, 2, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(texcoord_loc);

            let _maskros = Texture::load(gl, include_bytes!("maskros512.tga"), false);

            Demo { vao }
        }
    }

    fn display(&mut self, gl: &glow::Context, _window: &sdl2::video::Window) {
        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(TRIANGLES, 0, 6);
        }
    }
}

fn main() {
    Demo::sdl2_window("Textured quad example").start();
}
