#[macro_use]
extern crate load_file;

use std::collections::HashMap;

use fbo::SceneFBO;
use microglut::{
    delta_time,
    fbo::{bind_output_fbo, bind_texture_fbo},
    glam::{Mat4, Vec2, Vec3, Vec4},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, NativeTexture, NativeVertexArray,
        ARRAY_BUFFER, BLEND, CLAMP_TO_EDGE, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, FLOAT, FRAMEBUFFER,
        LINEAR, ONE_MINUS_SRC_ALPHA, RGBA, RGBA32F, SHADER_STORAGE_BUFFER, SRC_ALPHA, STATIC_DRAW,
        TEXTURE0, TEXTURE1, TEXTURE2, TEXTURE_2D_ARRAY, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER,
        TEXTURE_WRAP_S, TEXTURE_WRAP_T, TRIANGLES, UNSIGNED_BYTE,
    },
    load_shaders, MicroGLUT, Window, FBO,
};
use sprite::Sprite;

fn debug_message_callback(_source: u32, _type: u32, _id: u32, severity: u32, message: String) {
    let severity = match severity {
        DEBUG_SEVERITY_MEDIUM => "M",
        DEBUG_SEVERITY_HIGH => "H",
        _ => return,
    };
    eprintln!("[{severity}] {message}");
}

mod fbo;
mod sprite;

/// Rounds up a number to a power of n.
/// # Examples
/// ```
/// let x = 2.5;
/// let y = 4.0;
/// assert!(ceil_to_power_of_n(x, 2.0) == 4.0)
/// assert!(ceil_to_power_of_n(x, 2.0) == ceil_to_power_of_n(y, 2.0))
/// ```
fn ceil_to_power_of_n(number: f32, n: f32) -> f32 {
    n.powf(number.log(n).ceil())
}

/// Rounds up a numver to a multiple of n.
/// # Examples
/// ```
/// let x = 5.0;
/// let y = -4.0;
/// assert!(ceil_to_multiple_of_n(x, 4.0) == 8.0)
/// assert!(ceil_to_multiple_of_n(y, 4.0) == -4.0)
/// ```
fn ceil_to_multiple_of_n(number: f32, n: f32) -> f32 {
    (number / n).ceil() * n
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

    scene: SceneFBO,
    dist_field: FBO,
    prev_cascade: FBO,
    curr_cascade: FBO,

    screen_width: i32,
    screen_height: i32,
    cascade_width: f32,
    cascade_height: f32,
    probe_spacing: f32,
    interval_length: f32,

    texture_array: NativeTexture,
    sprites: Vec<Sprite>,
    sprite_ssbo: NativeBuffer, // Uses binding point 0
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.scene.fb));
            gl.use_program(Some(self.scene_program));
            gl.enable(BLEND);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.bind_vertex_array(Some(self.quad_vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(self.quad_vertex_buffer));
            let pos_loc = gl
                .get_attrib_location(self.scene_program, "position")
                .unwrap();
            gl.vertex_attrib_pointer_f32(pos_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(pos_loc);

            gl.bind_buffer(ARRAY_BUFFER, Some(self.quad_texcoord_buffer));
            if let Some(texcoord_loc) = gl.get_attrib_location(self.scene_program, "v_tex_coord") {
                gl.vertex_attrib_pointer_f32(texcoord_loc, 2, FLOAT, false, 0, 0);
                gl.enable_vertex_attrib_array(texcoord_loc);
            }

            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D_ARRAY, Some(self.texture_array));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.scene_program, "tex_array")
                    .as_ref(),
                0,
            );

            let sprite_block_index = gl
                .get_shader_storage_block_index(self.scene_program, "SpriteBuffer")
                .unwrap();
            gl.shader_storage_block_binding(self.scene_program, sprite_block_index, 0);

            gl.draw_arrays_instanced(TRIANGLES, 0, 3, self.sprites.len() as _);
            gl.disable(BLEND);
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
            self.scene.bind_as_textures(gl, TEXTURE0);
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
        let num_cascades = (Vec2::ZERO.distance(Vec2::new(
            self.screen_width as f32,
            self.screen_height as f32,
        )))
        .log(4.0)
        .ceil() as i32;
        unsafe {
            for n in (0..num_cascades).rev() {
                gl.use_program(Some(self.rc_program));
                bind_texture_fbo(gl, &self.prev_cascade, TEXTURE0);
                bind_texture_fbo(gl, &self.dist_field, TEXTURE1);
                self.scene.bind_as_textures(gl, TEXTURE2);

                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "scene_emissive")
                        .as_ref(),
                    3,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "scene_albedo")
                        .as_ref(),
                    2,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.rc_program, "dist_field")
                        .as_ref(),
                    1,
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
                    self.cascade_width as _,
                    self.cascade_height as _,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "num_cascades")
                        .as_ref(),
                    num_cascades as _,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "c0_probe_spacing")
                        .as_ref(),
                    self.probe_spacing,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "c0_interval_length")
                        .as_ref(),
                    self.interval_length,
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

        let screen_width = window.size().0 as i32;
        let screen_height = window.size().1 as i32;
        let texture_width = 200;
        let texture_height = 200;

        let textures = vec![
            load_bytes!("textures/RainTexture1.png"),
            load_bytes!("textures/SnowTexture2.png"),
            load_bytes!("textures/black2.png"),
            load_bytes!("textures/red_circle.png"),
            load_bytes!("textures/white_circle.png"),
        ];
        let tex_names = HashMap::from([
            ("rain", 0.0),
            ("snow", 1.0),
            ("wall", 2.0),
            ("red_circle", 3.0),
            ("white_circle", 4.0),
        ]); // Used for convenience when giving sprites textures
        let mut img = vec![];
        use stb_image::image::{load_from_memory, LoadResult};
        for i in 0..textures.len() {
            let image = match load_from_memory(textures[i]) {
                LoadResult::Error(e) => panic!("{}", e),
                LoadResult::ImageU8(image) => image,
                LoadResult::ImageF32(_image) => todo!(),
            };
            img.extend_from_slice(&image.data);
        }

        let sprites = vec![
            Sprite::new(
                *tex_names.get("rain").unwrap(),
                Vec2::ZERO,
                Vec2::ONE * 0.03,
                0.0,
                Vec4::new(0.0, 0.0, 1.0, 1.0),
            ),
            Sprite::new(
                *tex_names.get("snow").unwrap(),
                Vec2::new(0.3, 0.3),
                Vec2::ONE * 0.3,
                0.25,
                Vec4::ONE,
            ),
            Sprite::new(
                *tex_names.get("wall").unwrap(),
                Vec2::new(0.0, 0.3),
                Vec2::ONE * 0.03,
                0.0,
                Vec4::ZERO,
            ),
            Sprite::new(
                *tex_names.get("red_circle").unwrap(),
                Vec2::new(-0.3, -0.3),
                Vec2::ONE * 0.07,
                0.0,
                Vec4::new(0.0, 0.0, 0.0, 1.0),
            ),
        ];

        let probe_spacing = 2.0; // Should be some power of 2^N where N may be either positive or negative. Smaller N yields better quality
        let interval_length = Vec2::ZERO.distance(Vec2::new(probe_spacing, probe_spacing)) * 0.5;
        let probe_spacing_adjusted = ceil_to_power_of_n(probe_spacing, 2.0);
        let interval_length_adjusted = ceil_to_multiple_of_n(interval_length, 2.0);
        let cascade_width = (screen_width as f32) / probe_spacing_adjusted;
        let cascade_height = (screen_height as f32) / probe_spacing_adjusted;

        unsafe {
            let quad_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(quad_vao));

            let quad_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let quad_tex_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(quad_tex_vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&texcoords), STATIC_DRAW);

            // Load all shaders
            let scene_program = load_shaders(
                gl,
                include_str!("scene_vertex.glsl"),
                include_str!("scene_fragment.glsl"),
            );
            let rc_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("rc_bilinear.glsl"),
            );
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

            let dist_field = FBO::init(gl, screen_width, screen_height, false);
            let scene = SceneFBO::init(gl, screen_width, screen_height, 2);
            let prev_cascade = FBO::init(gl, cascade_width as _, cascade_height as _, false);
            let curr_cascade = FBO::init(gl, cascade_width as _, cascade_height as _, false);

            // Load sprite textures into a texture array
            let texture_array = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D_ARRAY, Some(texture_array));
            gl.tex_storage_3d(
                TEXTURE_2D_ARRAY,
                1,
                RGBA,
                texture_width,
                texture_height,
                textures.len() as _,
            );
            gl.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MIN_FILTER, LINEAR as _);
            gl.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MAG_FILTER, LINEAR as _);
            gl.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_WRAP_S, CLAMP_TO_EDGE as _);
            gl.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_WRAP_T, CLAMP_TO_EDGE as _);
            gl.tex_image_3d(
                TEXTURE_2D_ARRAY,
                0,
                RGBA32F as _,
                texture_width,
                texture_height,
                textures.len() as _,
                0,
                RGBA,
                UNSIGNED_BYTE,
                Some(&img),
            );

            let sprite_ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(sprite_ssbo));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::cast_slice(&sprites),
                STATIC_DRAW,
            );
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(sprite_ssbo));

            gl.viewport(0, 0, screen_width, screen_height);

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
                cascade_width,
                cascade_height,
                probe_spacing: probe_spacing_adjusted,
                interval_length: interval_length_adjusted,
                texture_array,
                sprites,
                sprite_ssbo,
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        let rot_mat = Mat4::from_rotation_z(delta_time());
        self.sprites[3].model_to_world = rot_mat * self.sprites[3].model_to_world;
        unsafe {
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                3 * size_of::<Sprite>() as i32,
                bytemuck::bytes_of(&self.sprites[3]),
            );
        }
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
        .window_size(1024, 1024)
        .start();
}
