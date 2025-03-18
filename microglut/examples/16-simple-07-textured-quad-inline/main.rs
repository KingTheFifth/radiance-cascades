use glam::{Vec2, Vec3};
use glow::{
    HasContext, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, DEPTH_TEST,
    FLOAT, LINEAR, REPEAT, RGB, STATIC_DRAW, TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER,
    TEXTURE_WRAP_S, TEXTURE_WRAP_T, TRIANGLES, UNSIGNED_BYTE,
};
use microglut::{load_shaders, MicroGLUT};

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
        let texture_data: [[[u8; 3]; 4]; 4] = [
            [[255, 50, 255], [50, 50, 255], [50, 50, 255], [50, 255, 255]],
            [[50, 50, 255], [255, 50, 255], [50, 255, 255], [50, 50, 255]],
            [[50, 50, 255], [50, 255, 255], [255, 50, 255], [50, 50, 255]],
            [[50, 255, 255], [50, 50, 255], [50, 50, 255], [255, 50, 255]],
        ];

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            let program = load_shaders(
                gl,
                include_str!("inlinetexture.vert"),
                include_str!("inlinetexture.frag"),
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

            let texture = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGB as _,
                4,
                4,
                0,
                RGB,
                UNSIGNED_BYTE,
                Some(bytemuck::cast_slice(&texture_data)),
            );

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
