use microglut::{
    fbo::{bind_output_fbo, bind_texture_fbo},
    glam::{Vec2, Vec3},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, NativeVertexArray, ARRAY_BUFFER,
        COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, STATIC_DRAW, TEXTURE0, TEXTURE1,
        TEXTURE2, TRIANGLES,
    },
    load_shaders, MicroGLUT, Window, FBO,
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
    //TODO: the "quad" is actually a triangle that covers the screen. Rename it accordingly?
    quad_vao: NativeVertexArray,
    quad_vertex_buffer: NativeBuffer,
    quad_texcoord_buffer: NativeBuffer,
    scene_program: NativeProgram,
    rc_program: NativeProgram,
    jfa_seed_program: NativeProgram,
    jfa_program: NativeProgram,
    sdf_program: NativeProgram,
    fbo_program: NativeProgram,
    scene: FBO,
    dist_field: FBO,
    prev_cascade: FBO,
    curr_cascade: FBO,
    screen_width: i32,
    screen_height: i32,
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            bind_output_fbo(gl, Some(&self.scene), self.screen_width, self.screen_height);
            gl.use_program(Some(self.scene_program));

            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            self.draw_screen_quad(gl, self.scene_program);
        }
    }

    fn draw_screen_quad(&self, gl: &Context, program: NativeProgram) {
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

    fn draw_fbo(&self, gl: &Context, source: &FBO, destination: Option<&FBO>) {
        unsafe {
            gl.use_program(Some(self.fbo_program));
            bind_texture_fbo(gl, source, TEXTURE0);
            bind_output_fbo(gl, destination, self.screen_width, self.screen_height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            self.draw_screen_quad(gl, self.fbo_program);
        }
    }

    fn calculate_dist_field(&mut self, gl: &Context) {
        unsafe {
            // Seed the jump flood algorithm
            let mut tmp = FBO::init(gl, self.screen_width, self.screen_height, false);
            gl.use_program(Some(self.jfa_seed_program));
            bind_texture_fbo(gl, &self.scene, TEXTURE0);
            bind_output_fbo(
                gl,
                Some(&self.dist_field),
                self.screen_width,
                self.screen_height,
            );
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.jfa_seed_program, "tex")
                    .as_ref(),
                0,
            );
            self.draw_screen_quad(gl, self.jfa_seed_program);

            // Jump flood algorithm
            gl.use_program(Some(self.jfa_program));

            gl.uniform_1_i32(gl.get_uniform_location(self.jfa_program, "tex").as_ref(), 0);
            gl.uniform_2_f32(
                gl.get_uniform_location(self.jfa_program, "screen_dimensions")
                    .as_ref(),
                self.screen_width as _,
                self.screen_height as _,
            );

            let passes = (self.screen_width.max(self.screen_height) as f32)
                .log2()
                .ceil() as u32;
            let mut ping_pong = false;
            for i in 0..passes {
                if ping_pong {
                    ping_pong = false;
                    bind_texture_fbo(gl, &tmp, TEXTURE0);
                    bind_output_fbo(
                        gl,
                        Some(&self.dist_field),
                        self.screen_width,
                        self.screen_height,
                    );
                } else {
                    ping_pong = true;
                    bind_texture_fbo(gl, &self.dist_field, TEXTURE0);
                    bind_output_fbo(gl, Some(&tmp), self.screen_width, self.screen_height);
                }

                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

                let jump_dist = (2 as i32).pow(passes - i - 1);
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.jfa_program, "jump_dist")
                        .as_ref(),
                    jump_dist as f32,
                );

                self.draw_screen_quad(gl, self.jfa_program);
            }

            if !ping_pong {
                self.draw_fbo(gl, &self.dist_field, Some(&tmp));
            }

            // Finalise the distance field
            gl.use_program(Some(self.sdf_program));
            bind_texture_fbo(gl, &tmp, TEXTURE0);
            bind_output_fbo(
                gl,
                Some(&self.dist_field),
                self.screen_width,
                self.screen_height,
            );
            gl.uniform_1_i32(gl.get_uniform_location(self.sdf_program, "tex").as_ref(), 0);
            gl.uniform_2_f32(
                gl.get_uniform_location(self.sdf_program, "screen_dimensions")
                    .as_ref(),
                self.screen_width as _,
                self.screen_height as _,
            );
            self.draw_screen_quad(gl, self.sdf_program);

            tmp.delete(gl);
        }
    }

    fn calculate_cascades(&mut self, gl: &Context) {
        let num_cascades = 4;
        let probe_density = 2.0;
        let interval_length = 1.0;
        unsafe {
            for n in (0..num_cascades).rev() {
                gl.use_program(Some(self.rc_program));
                bind_texture_fbo(gl, &self.prev_cascade, TEXTURE0);
                bind_texture_fbo(gl, &self.dist_field, TEXTURE2);
                bind_texture_fbo(gl, &self.scene, TEXTURE1);

                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "scene").as_ref(),
                    1,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "dist_field")
                        .as_ref(),
                    2,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "prev_cascade")
                        .as_ref(),
                    0,
                );
                gl.uniform_2_f32(
                    gl.get_uniform_location(self.rc_program, "screen_dimensions")
                        .as_ref(),
                    self.screen_width as _,
                    self.screen_height as _,
                );
                gl.uniform_2_f32(
                    gl.get_uniform_location(self.rc_program, "cascade_dimensions")
                        .as_ref(),
                    self.screen_width as _,
                    self.screen_height as _,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "num_cascades")
                        .as_ref(),
                    num_cascades as _,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "c0_probe_density")
                        .as_ref(),
                    probe_density,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "c0_interval_length")
                        .as_ref(),
                    interval_length,
                );

                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "cascade_index")
                        .as_ref(),
                    n as _,
                );

                bind_output_fbo(
                    gl,
                    Some(&self.curr_cascade),
                    self.screen_width,
                    self.screen_height,
                );

                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
                self.draw_screen_quad(gl, self.rc_program);

                self.draw_fbo(gl, &self.curr_cascade, Some(&self.prev_cascade));
            }
        }
    }
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
            let quad_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(quad_vao));

            let quad_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let quad_tex_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_tex_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&texcoords), STATIC_DRAW);

            let scene_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("scene_fragment.glsl"),
            );

            let rc_program = load_shaders(gl, include_str!("vertex.glsl"), include_str!("rc.glsl"));

            let jfa_seed_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("dist_field/seed_jump_flood.glsl"),
            );
            let jfa_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("dist_field/jump_flood.glsl"),
            );
            let sdf_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("dist_field/dist_field.glsl"),
            );

            let fbo_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("fbo_fragment.glsl"),
            );

            let screen_width = 1024;
            let screen_height = 1024;
            let dist_field = FBO::init(gl, screen_width, screen_height, false);
            let scene = FBO::init(gl, screen_width, screen_height, false);
            let prev_cascade = FBO::init(gl, screen_width, screen_height, false);
            let curr_cascade = FBO::init(gl, screen_width, screen_height, false);

            App {
                quad_vao,
                quad_vertex_buffer: quad_vbo,
                quad_texcoord_buffer: quad_tex_vbo,
                scene_program,
                rc_program,
                jfa_seed_program,
                jfa_program,
                sdf_program,
                fbo_program,
                scene,
                dist_field,
                prev_cascade,
                curr_cascade,
                screen_width,
                screen_height,
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        self.draw_scene(gl);
        self.calculate_dist_field(gl);
        self.calculate_cascades(gl);
        self.draw_fbo(gl, &self.curr_cascade, None);
    }
}

fn main() {
    App::sdl2_window("Radiance cascades 2D prototype")
        .gl_version(4, 5)
        .debug_message_callback(debug_message_callback)
        .start();
}
