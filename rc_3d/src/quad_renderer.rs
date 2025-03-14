use microglut::{
    glam::{Vec2, Vec3},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, NativeVertexArray, ARRAY_BUFFER, FLOAT,
        STATIC_DRAW, TRIANGLES,
    },
};

pub struct QuadRenderer {
    quad_vao: NativeVertexArray,
    quad_vertex_buffer: NativeBuffer,
    quad_texcoord_buffer: NativeBuffer,
}

impl QuadRenderer {
    pub fn new(gl: &Context) -> Self {
        let vertices = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(3.0, -1.0, 0.0),
            Vec3::new(-1.0, 3.0, 0.0),
        ];

        let texcoords = [Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0)];
        unsafe {
            let quad_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(quad_vao));

            let quad_vertex_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_vertex_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let quad_texcoord_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_texcoord_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&texcoords), STATIC_DRAW);

            Self {
                quad_vao,
                quad_vertex_buffer,
                quad_texcoord_buffer,
            }
        }
    }
    pub fn draw_screen_quad(&self, gl: &Context, program: NativeProgram) {
        unsafe {
            gl.bind_vertex_array(Some(self.quad_vao));

            gl.bind_buffer(ARRAY_BUFFER, Some(self.quad_vertex_buffer));
            let pos_loc = gl.get_attrib_location(program, "position").unwrap();
            gl.vertex_attrib_pointer_f32(pos_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(pos_loc);

            gl.bind_buffer(ARRAY_BUFFER, Some(self.quad_texcoord_buffer));
            if let Some(texcoord_loc) = gl.get_attrib_location(program, "v_tex_coord") {
                gl.vertex_attrib_pointer_f32(texcoord_loc, 2, FLOAT, false, 0, 0);
                gl.enable_vertex_attrib_array(texcoord_loc);
            }

            gl.draw_arrays(TRIANGLES, 0, 3);
        }
    }
}
