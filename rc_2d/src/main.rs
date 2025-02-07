use microglut::{
    glam::{Vec2, Vec3},
    glow::{
        Context, HasContext, NativeProgram, NativeVertexArray, ARRAY_BUFFER, COLOR_BUFFER_BIT,
        DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, STATIC_DRAW, TRIANGLES,
    },
    load_shaders, MicroGLUT, Texture, Window,
};

fn debug_message_callback(_source: u32, _type: u32, _id: u32, severity: u32, message: String) {
    let severity = match severity {
        DEBUG_SEVERITY_MEDIUM => "M",
        DEBUG_SEVERITY_HIGH => "H",
        _ => return,
    };
    eprintln!("[{severity}] {message}");
}

struct App {
    program: NativeProgram,
    vao: NativeVertexArray,
}

impl MicroGLUT for App {
    fn init(gl: &Context, window: &Window) -> Self {
        let vertices = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(3.0, -1.0, 0.0),
            Vec3::new(-1.0, 3.0, 0.0),
        ];

        let texcoords = [Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0)];

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.enable(DEPTH_TEST);
            let program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("fragment.glsl"),
            );
            gl.use_program(Some(program));

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let pos_loc = gl.get_attrib_location(program, "position").unwrap();
            gl.vertex_attrib_pointer_f32(pos_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(pos_loc);

            let texcoord_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(texcoord_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&texcoords), STATIC_DRAW);

            let texcoord_loc = gl.get_attrib_location(program, "tex_coord").unwrap();
            gl.vertex_attrib_pointer_f32(texcoord_loc, 2, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(texcoord_loc);

            let _texture_data = Texture::load(
                gl,
                include_bytes!(
                    "../../microglut/examples/16-simple-06-textured-quad/maskros512.tga"
                ),
                false,
            );

            App { program, vao }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(TRIANGLES, 0, 3);
        }
    }
}

fn main() {
    App::sdl2_window("Radiance cascades 2D prototype")
        .gl_version(4, 5)
        .debug_message_callback(debug_message_callback)
        .start();
}
