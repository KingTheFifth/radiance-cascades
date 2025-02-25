#[macro_use]
extern crate load_file;

use std::f32::consts::PI;

use fbo::SceneFBO;
use microglut::{
    fbo::{bind_output_fbo, bind_texture_fbo},
    glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, NativeVertexArray, ARRAY_BUFFER, BLEND,
        COLOR_ATTACHMENT0, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, FRAMEBUFFER,
        ONE_MINUS_SRC_ALPHA, SRC_ALPHA, STATIC_DRAW, TEXTURE0, TEXTURE1, TEXTURE2, TEXTURE_2D,
        TEXTURE_MAX_LEVEL, TRIANGLES,
    },
    load_shaders, MicroGLUT, Model, Window, FBO,
};
use object::Object;

fn debug_message_callback(_source: u32, _type: u32, _id: u32, severity: u32, message: String) {
    let severity = match severity {
        DEBUG_SEVERITY_MEDIUM => "M",
        DEBUG_SEVERITY_HIGH => "H",
        _ => return,
    };
    eprintln!("[{severity}] {message}");
}

mod fbo;
mod object;

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
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            let cam_pos = Vec3::ZERO;
            let cam_look_at = -Vec3::Z;
            let fov = PI / 2.0;
            let focus_dist = 2.0;
            let aspect_ratio = self.screen_width as f32 / self.screen_height as f32;
            let cam_forward = (cam_pos - cam_look_at).normalize();
            let mut cam_up = Vec3::Y;
            let cam_right = cam_up.cross(cam_forward).normalize();
            cam_up = cam_forward.cross(cam_right);

            let w_t_v = Mat4::look_at_rh(cam_pos, cam_look_at, Vec3::Y);
            let z_near = -0.1;
            let z_far = -20.0;
            let perspective_mat = Mat4::perspective_rh(fov, aspect_ratio, -z_near, -z_far);

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
                object
                    .model
                    .draw(gl, self.scene_program, "position", None, None);
            }

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
            gl.use_program(Some(self.depth_program));
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.depth_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.depth_program, "depth_tex")
                    .as_ref(),
                0,
            );
            gl.framebuffer_texture(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
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
                let mip_dims = start_dims / 2.0_f32.powi(level);
                let prev_dims = start_dims / 2.0_f32.powi(level - 1);

                gl.framebuffer_texture(
                    FRAMEBUFFER,
                    COLOR_ATTACHMENT0,
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

            // Restore the original texture attachment to color attachment 0
            gl.bind_framebuffer(TEXTURE_2D, Some(self.scene.fb));
            gl.framebuffer_texture(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
                Some(self.scene.textures[0]),
                0,
            );
        }
    }

    fn draw_ssrt(&self, gl: &Context) {}

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
            let depth_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("../shaders/min_max.frag"),
            );
            let ssrt_program =
                load_shaders(gl, include_str!("vertex.glsl"), include_str!("test.frag"));
            let rc_program = load_shaders(gl, include_str!("vertex.glsl"), include_str!("rc.glsl"));
            let fbo_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("fbo_fragment.glsl"),
            );

            let dist_field = FBO::init(gl, screen_width, screen_height, false);
            let scene = SceneFBO::init(gl, screen_width, screen_height, 2);
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
                    .with_translation(Vec3::new(0.0, 0.0, -2.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(1.0))
                    .with_translation(Vec3::new(-0.5, 0.0, -1.0)),
            ];

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
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        self.draw_scene(gl);
        // unsafe {
        //     gl.bind_framebuffer(FRAMEBUFFER, None);
        // }
        // self.draw_ssrt(gl);
        self.generate_hi_z_buffer(gl);
        //unsafe {
        //    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
        //    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
        //    gl.blit_framebuffer(
        //        0,
        //        0,
        //        self.screen_width,
        //        self.screen_height,
        //        0,
        //        0,
        //        self.screen_width,
        //        self.screen_height,
        //        COLOR_BUFFER_BIT,
        //        LINEAR,
        //    );
        //}
        //unsafe {
        //    gl.use_program(Some(self.fbo_program));
        //    gl.active_texture(TEXTURE0);
        //    gl.bind_texture(TEXTURE_2D, Some(self.scene.depth_texture));
        //    gl.bind_framebuffer(FRAMEBUFFER, None);
        //    gl.clear_color(0.0, 0.0, 0.0, 0.0);
        //    gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        //    self.draw_screen_quad(gl, self.fbo_program);
        //}

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
