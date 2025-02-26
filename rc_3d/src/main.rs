#[macro_use]
extern crate load_file;

use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use microglut::{
    fbo::{bind_output_fbo, bind_texture_fbo},
    glam::{Mat4, Quat, Vec2, Vec3, Vec4},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, NativeVertexArray, ARRAY_BUFFER, BLEND,
        COLOR_ATTACHMENT0, COLOR_ATTACHMENT3, COLOR_ATTACHMENT4, COLOR_BUFFER_BIT, DEBUG_OUTPUT,
        DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, FRAMEBUFFER, MAX_COLOR_ATTACHMENTS,
        ONE_MINUS_SRC_ALPHA, SHADER_STORAGE_BUFFER, SRC_ALPHA, STATIC_DRAW, TEXTURE0, TEXTURE1,
        TEXTURE2, TEXTURE_2D, TEXTURE_MAX_LEVEL, TRIANGLES,
    },
    load_shaders, MicroGLUT, Model, Window, FBO,
};
use object::Object;
use scene_fbo::SceneFBO;

fn debug_message_callback(_source: u32, _type: u32, _id: u32, severity: u32, message: String) {
    let severity = match severity {
        DEBUG_SEVERITY_MEDIUM => "M",
        DEBUG_SEVERITY_HIGH => "H",
        _ => return,
    };
    eprintln!("[{severity}] {message}");
}

mod object;
mod scene_fbo;

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

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
struct HiZConstants {
    pub screen_res: Vec2,
    pub screen_res_inv: Vec2,
    pub hi_z_resolution: Vec2,
    pub inv_hi_z_resolution: Vec2,
    pub hi_z_start_mip_level: f32,
    pub hi_z_max_mip_level: f32,

    pub max_steps: f32,
    pub z_far: f32,

    pub perspective: Mat4,
    pub perspective_inv: Mat4,
    pub viewport: Mat4,
    pub viewport_inv: Mat4,
    pub z_near: f32,
    pub max_ray_distance: f32,
    _padding: [f32; 2],
}

struct App {
    //TODO: the "quad" is actually a triangle that covers the screen. Rename it accordingly?
    quad_vao: NativeVertexArray,
    quad_vertex_buffer: NativeBuffer,
    quad_texcoord_buffer: NativeBuffer,

    scene_program: NativeProgram,
    depth_program: NativeProgram,
    ssrt_program: NativeProgram,
    rc_program: NativeProgram,
    fbo_program: NativeProgram,

    objects: Vec<Object>,
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
    constants_ssbo: NativeBuffer,
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            let cam_pos = Vec3::ZERO;
            let cam_look_at = -Vec3::Z;
            let fov = PI / 2.0;
            let aspect_ratio = self.screen_width as f32 / self.screen_height as f32;

            let w_t_v = Mat4::look_at_rh(cam_pos, cam_look_at, Vec3::Y);
            let z_near = -0.1;
            let z_far = -20.0;
            let perspective_mat = Mat4::perspective_rh(fov, aspect_ratio, -z_near, -z_far);

            let w_2 = self.screen_width as f32 / 2.0;
            let h_2 = self.screen_height as f32 / 2.0;
            #[rustfmt::skip]
            let viewport_mat = Mat4::from_cols_array(&[
                w_2, 0.0, 0.0, w_2,
                0.0, h_2, 0.0, h_2,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ]).transpose();

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(self.constants_ssbo));
            gl.buffer_sub_data_u8_slice(SHADER_STORAGE_BUFFER, 0x2C, bytemuck::bytes_of(&-z_far));
            gl.buffer_sub_data_u8_slice(SHADER_STORAGE_BUFFER, 0x130, bytemuck::bytes_of(&-z_near));
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0x30,
                bytemuck::bytes_of(&perspective_mat),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0x70,
                bytemuck::bytes_of(&perspective_mat.inverse()),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0xB0,
                bytemuck::bytes_of(&viewport_mat),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0xF0,
                bytemuck::bytes_of(&viewport_mat.inverse()),
            );
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            gl.bind_framebuffer(FRAMEBUFFER, Some(self.scene.fb));
            gl.use_program(Some(self.scene_program));
            gl.enable(BLEND);
            gl.enable(DEPTH_TEST);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            gl.clear_color(0.0, 0.5, 0.5, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.scene_program, "world_to_view")
                    .as_ref(),
                false,
                w_t_v.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.scene_program, "projection")
                    .as_ref(),
                false,
                perspective_mat.as_ref(),
            );

            for object in &self.objects {
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.scene_program, "model_to_world")
                        .as_ref(),
                    false,
                    object.get_transformation().as_ref(),
                );
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.scene_program, "v_albedo")
                        .as_ref(),
                    object.albedo.as_ref(),
                );
                object
                    .model
                    .draw(gl, self.scene_program, "position", Some("v_normal"), None);
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.disable(BLEND);
            gl.disable(DEPTH_TEST);
        }
    }

    fn generate_hi_z_buffer(&self, gl: &Context) {
        let start_dims = Vec2::new(self.screen_width as _, self.screen_height as _);
        let max_mip_level = self.screen_width.max(self.screen_height).ilog2() as i32;
        unsafe {
            // Mip-map level 0 separately with the scene depth buffer as input
            // to properly populate the first level of min & max depth
            // Note: It is possible to write the level 0 data in the shader for the scene,
            // but this fills level 0 with the scene clear colour for any holes in the scene
            gl.use_program(Some(self.depth_program));
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.scene.fb));
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.depth_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.depth_program, "depth_tex")
                    .as_ref(),
                0,
            );
            gl.framebuffer_texture(
                FRAMEBUFFER,
                COLOR_ATTACHMENT3,
                Some(self.scene.hi_z_texture),
                0,
            );
            gl.uniform_2_f32_slice(
                gl.get_uniform_location(self.depth_program, "dimensions")
                    .as_ref(),
                start_dims.as_ref(),
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.depth_program, "level")
                    .as_ref(),
                0,
            );
            self.draw_screen_quad(gl, self.depth_program);

            // Calculate each mip-level using the previous one as the input
            gl.bind_texture(TEXTURE_2D, Some(self.scene.hi_z_texture));
            for level in 1..max_mip_level + 1 {
                let mip_dims = (start_dims / 2.0_f32.powi(level)).max(Vec2::new(1.0, 1.0));
                let prev_dims = (start_dims / 2.0_f32.powi(level - 1)).max(Vec2::new(1.0, 1.0));

                gl.framebuffer_texture(
                    FRAMEBUFFER,
                    COLOR_ATTACHMENT3,
                    Some(self.scene.hi_z_texture),
                    level,
                );
                gl.uniform_2_f32_slice(
                    gl.get_uniform_location(self.depth_program, "dimensions")
                        .as_ref(),
                    mip_dims.as_ref(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.depth_program, "level")
                        .as_ref(),
                    level,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.depth_program, "prev_mip_level")
                        .as_ref(),
                    level - 1,
                );
                gl.uniform_2_f32_slice(
                    gl.get_uniform_location(self.depth_program, "prev_level_dimensions")
                        .as_ref(),
                    prev_dims.as_ref(),
                );

                // Prevent reading the current mip-level as that would be undefined behaviour
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAX_LEVEL, level - 1);

                gl.viewport(0, 0, mip_dims.x as _, mip_dims.y as _);
                self.draw_screen_quad(gl, self.depth_program);
            }

            // Restore to original value
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAX_LEVEL, 1000);
            gl.viewport(0, 0, self.screen_width, self.screen_height);
            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.bind_texture(TEXTURE_2D, None);
        }
    }

    fn draw_ssrt(&self, gl: &Context) {
        unsafe {
            gl.use_program(Some(self.ssrt_program));
            gl.bind_framebuffer(FRAMEBUFFER, None);

            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.hi_z_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.ssrt_program, "hi_z_tex")
                    .as_ref(),
                0,
            );
            gl.active_texture(TEXTURE1);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.albedo));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.ssrt_program, "scene_albedo")
                    .as_ref(),
                1,
            );
            gl.active_texture(TEXTURE2);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.normal));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.ssrt_program, "scene_normal")
                    .as_ref(),
                2,
            );

            let constants_ssbo_loc = gl
                .get_shader_storage_block_index(self.ssrt_program, "HiZConstants")
                .unwrap();
            gl.shader_storage_block_binding(self.ssrt_program, constants_ssbo_loc, 0);
            self.draw_screen_quad(gl, self.ssrt_program);
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

        let probe_spacing = 2.0; // Should be some power of 2^N where N may be either positive or negative. Smaller N yields better quality
        let interval_length = Vec2::ZERO.distance(Vec2::new(probe_spacing, probe_spacing)) * 0.5;
        let probe_spacing_adjusted = ceil_to_power_of_n(probe_spacing, 2.0);
        let interval_length_adjusted = ceil_to_multiple_of_n(interval_length, 2.0);
        let cascade_width = (screen_width as f32) / probe_spacing_adjusted;
        let cascade_height = (screen_height as f32) / probe_spacing_adjusted;

        let screen_dims = Vec2::new(screen_width as _, screen_height as _);
        let constants = HiZConstants {
            screen_res: screen_dims,
            screen_res_inv: 1.0 / screen_dims,
            hi_z_resolution: screen_dims,
            inv_hi_z_resolution: 1.0 / screen_dims,
            hi_z_start_mip_level: 5.0,
            hi_z_max_mip_level: 10.0,
            max_steps: 500.0,
            z_near: 0.0,
            z_far: 0.0,
            perspective: Mat4::IDENTITY,
            perspective_inv: Mat4::IDENTITY,
            viewport: Mat4::IDENTITY,
            viewport_inv: Mat4::IDENTITY,
            max_ray_distance: 20.0,
            _padding: [0.0, 0.0],
        };

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
            let scene_program =
                load_shaders(gl, include_str!("scene.vert"), include_str!("scene.frag"));
            let depth_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("../shaders/min_max.frag"),
            );
            let ssrt_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("../shaders/hi_z_trace.frag"),
            );
            let rc_program = load_shaders(gl, include_str!("vertex.glsl"), include_str!("rc.glsl"));
            let fbo_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("fbo_fragment.glsl"),
            );

            let dist_field = FBO::init(gl, screen_width, screen_height, false);
            let scene = SceneFBO::init(gl, screen_width, screen_height);
            let prev_cascade = FBO::init(gl, cascade_width as _, cascade_height as _, false);
            let curr_cascade = FBO::init(gl, cascade_width as _, cascade_height as _, false);

            gl.viewport(0, 0, screen_width, screen_height);

            let rock = Model::load_obj_data(
                gl,
                include_bytes!("../models/Rock.obj"),
                Some(&|_| tobj::load_mtl_buf(&mut &include_bytes!("../models/Rock.mtl")[..])),
                None,
            );

            let objects = vec![
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(0.2))
                    .with_translation(Vec3::new(0.0, 0.0, -2.0))
                    .with_albedo(Vec4::new(1.0, 0.2, 0.8, 1.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(1.0))
                    .with_translation(Vec3::new(-0.5, 0.0, -1.0))
                    .with_albedo(Vec4::new(0.0, 0.5, 0.8, 1.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(PI))
                    .with_uniform_scale(15.0)
                    .with_translation(Vec3::new(0.0, -0.5, -3.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(3.0 * PI / 2.0))
                    .with_uniform_scale(15.0)
                    .with_translation(Vec3::new(0.0, 0.0, -6.0))
                    .with_albedo(Vec4::new(0.5, 0.1, 0.5, 1.0)),
            ];

            let constants_ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(constants_ssbo));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::bytes_of(&constants),
                STATIC_DRAW,
            );
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(constants_ssbo));
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            App {
                quad_vao,
                quad_vertex_buffer: quad_vbo,
                quad_texcoord_buffer: quad_tex_vbo,
                scene_program,
                depth_program,
                ssrt_program,
                rc_program,
                fbo_program,
                objects,
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
                constants_ssbo,
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        self.draw_scene(gl);
        self.generate_hi_z_buffer(gl);
        self.draw_ssrt(gl);

        // unsafe {
        //     gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
        //     gl.read_buffer(COLOR_ATTACHMENT0);
        //     gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
        //     gl.blit_framebuffer(
        //         0,
        //         0,
        //         self.screen_width,
        //         self.screen_height,
        //         0,
        //         0,
        //         self.screen_width,
        //         self.screen_height,
        //         COLOR_BUFFER_BIT,
        //         LINEAR,
        //     );
        // }
        // unsafe {
        //     gl.use_program(Some(self.fbo_program));
        //     gl.active_texture(TEXTURE0);
        //     gl.bind_texture(TEXTURE_2D, Some(self.scene.normal));
        //     gl.bind_framebuffer(FRAMEBUFFER, None);
        //     gl.clear_color(0.0, 0.0, 0.0, 0.0);
        //     gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        //     gl.uniform_1_i32(gl.get_uniform_location(self.fbo_program, "tex").as_ref(), 0);
        //     self.draw_screen_quad(gl, self.fbo_program);
        // }

        // self.calculate_cascades(gl);
        //self.draw_fbo(gl, &self.scene, None);
    }
}

fn main() {
    App::sdl2_window("Radiance cascades 3D prototype")
        .gl_version(4, 5)
        .debug_message_callback(debug_message_callback)
        .window_size(1024, 1024)
        .start();
}
