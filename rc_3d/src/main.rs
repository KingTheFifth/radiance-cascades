#[macro_use]
extern crate load_file;

use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use camera::Camera;
use cascade_fbo::CascadeFBO;
use microglut::{
    delta_time, elapsed_time,
    glam::{Mat4, Quat, Vec2, Vec3, Vec4},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, BLEND, COLOR_ATTACHMENT0,
        COLOR_ATTACHMENT3, COLOR_BUFFER_BIT, CULL_FACE, DEBUG_SEVERITY_LOW,
        DEBUG_SEVERITY_NOTIFICATION, DEPTH_BUFFER_BIT, DEPTH_TEST, DRAW_FRAMEBUFFER, FRAMEBUFFER,
        LINEAR, MULTISAMPLE, ONE_MINUS_SRC_ALPHA, READ_FRAMEBUFFER, SHADER_STORAGE_BUFFER,
        SRC_ALPHA, STATIC_DRAW, TEXTURE0, TEXTURE1, TEXTURE2, TEXTURE3, TEXTURE_2D,
        TEXTURE_MAX_LEVEL,
    },
    imgui, load_shaders,
    sdl2::{
        keyboard::{Keycode, Mod, Scancode},
        mouse::MouseButton,
    },
    MicroGLUT, Model, Window,
};
use object::Object;
use quad_renderer::QuadRenderer;
use scene_fbo::SceneFBO;
use voxelizer::Voxelizer;

fn debug_message_callback(_source: u32, _type: u32, _id: u32, severity: u32, message: String) {
    let severity = match severity {
        DEBUG_SEVERITY_MEDIUM => "M",
        DEBUG_SEVERITY_HIGH => "H",
        _ => return,
    };
    eprintln!("[{severity}] {message}");
}

mod camera;
mod cascade_fbo;
mod object;
mod quad_renderer;
mod scene_fbo;
mod voxelizer;

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
    pub z_near: f32,
    pub max_ray_distance: f32,
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
struct Constants {
    pub screen_res: Vec2,
    pub screen_res_inv: Vec2,

    /// Hi Z screen-space ray marching
    pub hi_z_resolution: Vec2,
    pub inv_hi_z_resolution: Vec2,

    pub world_to_view: Mat4,
    pub world_to_view_inv: Mat4,
    pub perspective: Mat4,
    pub perspective_inv: Mat4,

    pub hi_z_start_mip_level: f32,
    pub hi_z_max_mip_level: f32,
    pub max_steps: f32,
    pub max_ray_distance: f32,

    pub z_far: f32,
    pub z_near: f32,

    // Radiance cascades
    pub num_cascades: f32,
    pub c0_probe_spacing: f32,
    pub c0_interval_length: f32,
    _padding: [f32; 1],
    pub c0_resolution: Vec2,
}

enum DebugMode {
    RadianceCascades,
    RayMarching,
    DepthBuffer,
    Scene,
    Voxel,
}

struct App {
    scene_program: NativeProgram,
    depth_program: NativeProgram,
    ssrt_program: NativeProgram,
    rc_program: NativeProgram,
    post_pass_program: NativeProgram,

    objects: Vec<Object>,
    scene: SceneFBO,
    cascades: CascadeFBO,

    screen_width: i32,
    screen_height: i32,
    constants: Constants,
    constants_ssbo: NativeBuffer,

    camera: Camera,

    debug_cascade_index: i32,
    debug_cascades_merged: bool,
    debug: bool,
    debug_mode: DebugMode,
    debug_mode_idx: usize,

    mouse_is_down: bool,

    quad_renderer: QuadRenderer,
    voxelizer: Voxelizer,
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            let aspect_ratio = self.screen_width as f32 / self.screen_height as f32;

            let w_t_v = self.camera.view_transform();
            let perspective_mat = self.camera.perspective_transform(aspect_ratio);

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(self.constants_ssbo));
            gl.buffer_sub_data_u8_slice(SHADER_STORAGE_BUFFER, 0x20, bytemuck::bytes_of(&w_t_v));
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0x60,
                bytemuck::bytes_of(&w_t_v.inverse()),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0xA0,
                bytemuck::bytes_of(&perspective_mat),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0xE0,
                bytemuck::bytes_of(&perspective_mat.inverse()),
            );
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            gl.bind_framebuffer(FRAMEBUFFER, Some(self.scene.fb));
            gl.use_program(Some(self.scene_program));
            gl.enable(BLEND);
            gl.enable(DEPTH_TEST);
            gl.enable(CULL_FACE);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            gl.viewport(0, 0, self.screen_width, self.screen_height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
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
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.scene_program, "v_emissive")
                        .as_ref(),
                    object.emissive.as_ref(),
                );
                object
                    .model
                    .draw(gl, self.scene_program, "position", Some("v_normal"), None);
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.disable(BLEND);
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);
        }
    }

    fn generate_hi_z_buffer(&self, gl: &Context) {
        let start_dims = self.constants.hi_z_resolution;
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

            gl.viewport(0, 0, start_dims.x as _, start_dims.y as _);
            self.quad_renderer.draw_screen_quad(gl, self.depth_program);

            // Calculate each mip-level using the previous one as the input
            gl.bind_texture(TEXTURE_2D, Some(self.scene.hi_z_texture));
            for level in 1..(self.constants.hi_z_max_mip_level as i32) + 1 {
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
                self.quad_renderer.draw_screen_quad(gl, self.depth_program);
            }

            // Restore to original value
            gl.framebuffer_texture(
                FRAMEBUFFER,
                COLOR_ATTACHMENT3,
                Some(self.scene.hi_z_texture),
                0,
            );
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
                .get_shader_storage_block_index(self.ssrt_program, "Constants")
                .unwrap();
            gl.shader_storage_block_binding(self.ssrt_program, constants_ssbo_loc, 0);

            gl.viewport(0, 0, self.screen_width, self.screen_height);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            self.quad_renderer.draw_screen_quad(gl, self.ssrt_program);
        }
    }

    fn calculate_cascades(&mut self, gl: &Context) {
        unsafe {
            gl.use_program(Some(self.rc_program));
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.cascades.fb));
            gl.active_texture(TEXTURE1);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.albedo));
            gl.active_texture(TEXTURE2);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.emissive));
            gl.active_texture(TEXTURE3);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.hi_z_texture));

            gl.uniform_1_i32(
                gl.get_uniform_location(self.rc_program, "prev_cascade")
                    .as_ref(),
                0,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.rc_program, "scene_albedo")
                    .as_ref(),
                1,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.rc_program, "scene_emissive")
                    .as_ref(),
                2,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.rc_program, "hi_z_tex")
                    .as_ref(),
                3,
            );

            gl.uniform_1_i32(
                gl.get_uniform_location(self.rc_program, "merge_cascades")
                    .as_ref(),
                self.debug_cascades_merged.into(),
            );

            let constants_ssbo_loc = gl
                .get_shader_storage_block_index(self.rc_program, "Constants")
                .unwrap();
            gl.shader_storage_block_binding(self.rc_program, constants_ssbo_loc, 0);

            for n in (0..self.constants.num_cascades as i32).rev() {
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.rc_program, "cascade_index")
                        .as_ref(),
                    n as _,
                );

                gl.active_texture(TEXTURE0);
                self.cascades.bind_cascade_as_texture(
                    gl,
                    (n + 1).min(self.constants.num_cascades as i32 - 1) as _,
                    TEXTURE0,
                );

                gl.viewport(
                    0,
                    0,
                    self.constants.c0_resolution.x as _,
                    (self.constants.c0_resolution.y / 2.0_f32.powi(n)) as _,
                );
                self.cascades.bind_cascade_as_output(gl, n as _);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(COLOR_BUFFER_BIT);
                self.quad_renderer.draw_screen_quad(gl, self.rc_program);
            }

            gl.viewport(0, 0, self.screen_width, self.screen_height);
            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    fn integrate_radiance(&mut self, gl: &Context) {
        unsafe {
            gl.use_program(Some(self.post_pass_program));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.post_pass_program, "merged_cascade_0")
                    .as_ref(),
                0,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.post_pass_program, "scene_normal")
                    .as_ref(),
                1,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.post_pass_program, "scene_albedo")
                    .as_ref(),
                2,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.post_pass_program, "scene_emissive")
                    .as_ref(),
                3,
            );
            let constants_ssbo_loc = gl
                .get_shader_storage_block_index(self.post_pass_program, "Constants")
                .unwrap();
            gl.shader_storage_block_binding(self.post_pass_program, constants_ssbo_loc, 0);

            self.cascades.bind_cascade_as_texture(gl, 0, TEXTURE0);
            gl.active_texture(TEXTURE1);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.normal));
            gl.active_texture(TEXTURE2);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.albedo));
            gl.active_texture(TEXTURE3);
            gl.bind_texture(TEXTURE_2D, Some(self.scene.emissive));

            gl.viewport(0, 0, self.screen_width, self.screen_height);
            gl.clear(COLOR_BUFFER_BIT);
            self.quad_renderer
                .draw_screen_quad(gl, self.post_pass_program);
        }
    }
}

impl MicroGLUT for App {
    fn init(gl: &Context, window: &Window) -> Self {
        let screen_width = window.size().0 as i32;
        let screen_height = window.size().1 as i32;
        let screen_dims = Vec2::new(screen_width as _, screen_height as _);

        let probe_spacing = 2.0; // Should be some power of 2^N where N may be either positive or negative. Smaller N yields better quality
        let interval_length = Vec2::ZERO.distance(Vec2::new(probe_spacing, probe_spacing)) * 0.5;
        let probe_spacing_adjusted = ceil_to_power_of_n(probe_spacing, 2.0);
        let interval_length_adjusted = ceil_to_multiple_of_n(interval_length, 2.0);
        let cascade_width = 4.0 * (screen_width as f32) / probe_spacing_adjusted;
        let cascade_height = 4.0 * (screen_height as f32) / probe_spacing_adjusted;
        //let num_cascades = Vec2::ZERO.distance(screen_dims).log(4.0).ceil();
        let num_cascades = 4.0;

        let constants = Constants {
            screen_res: screen_dims,
            screen_res_inv: 1.0 / screen_dims,

            hi_z_resolution: screen_dims,
            inv_hi_z_resolution: 1.0 / screen_dims,
            world_to_view: Mat4::IDENTITY,
            world_to_view_inv: Mat4::IDENTITY,
            perspective: Mat4::IDENTITY,
            perspective_inv: Mat4::IDENTITY,
            hi_z_start_mip_level: 0.0,
            hi_z_max_mip_level: screen_width.max(screen_height).ilog2() as f32,
            max_steps: 400.0,
            max_ray_distance: 30.0,
            z_far: 20.0,
            z_near: 0.1,

            num_cascades,
            c0_probe_spacing: probe_spacing_adjusted,
            c0_interval_length: interval_length_adjusted,
            _padding: [0.0],
            c0_resolution: Vec2::new(cascade_width, cascade_height),
        };

        unsafe {
            gl.enable(MULTISAMPLE);

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
                include_str!("../shaders/naive_ray_marcher.frag"),
            );
            let rc_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("../shaders/rc.frag"),
            );
            let post_pass_program = load_shaders(
                gl,
                include_str!("vertex.glsl"),
                include_str!("../shaders/post_pass.frag"),
            );

            let scene = SceneFBO::init(gl, screen_width, screen_height);
            let cascades = CascadeFBO::new(gl, constants.c0_resolution, num_cascades as _);

            gl.viewport(0, 0, screen_width, screen_height);

            let rock = Model::load_obj_data(
                gl,
                include_bytes!("../models/Rock.obj"),
                Some(&|_| tobj::load_mtl_buf(&mut &include_bytes!("../models/Rock.mtl")[..])),
                None,
            );

            let objects = vec![
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(-0.2))
                    .with_translation(Vec3::new(0.0, 0.0, 2.0))
                    .with_albedo(Vec4::new(0.0, 0.0, 0.0, 1.0))
                    .with_emissive(Vec4::new(4.0, 4.0, 4.0, 1.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(-1.0))
                    .with_translation(Vec3::new(0.5, 0.0, 1.0))
                    .with_albedo(Vec4::new(0.0, 0.5, 0.8, 1.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(-PI))
                    .with_uniform_scale(15.0)
                    .with_translation(Vec3::new(0.0, -0.5, 3.0)),
                Object::new(rock.clone())
                    .with_rotation(Quat::from_rotation_x(-3.0 * PI / 2.0))
                    .with_uniform_scale(15.0)
                    .with_translation(Vec3::new(0.0, 0.0, 6.0))
                    .with_albedo(Vec4::new(0.5, 0.1, 0.5, 1.0)),
            ];
            //let objects = vec![Object::new(rock.clone())
            //    .with_albedo(Vec4::ONE)
            //    .with_translation(Vec3::new(0.0, 0.0, 1.0))
            //    .with_uniform_scale(2.0)];

            let constants_ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(constants_ssbo));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::bytes_of(&constants),
                STATIC_DRAW,
            );
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(constants_ssbo));
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            let quad_renderer = QuadRenderer::new(gl);
            //let voxelizer = Voxelizer::new(gl, Vec3::new(128., 128.0, 128.0));
            let voxel_res = 128.0;
            let voxel_origin = Vec3::new(0.0, 0.0, 4.5);
            let voxel_volume_half_side = 6.0;
            let voxelizer = Voxelizer::new(
                gl,
                Vec3::new(voxel_res, voxel_res, voxel_res),
                voxel_origin,
                voxel_volume_half_side,
            );
            voxelizer.clear_voxels(gl, &quad_renderer, Vec4::new(0., 0., 0., 0.0));
            let camera = Camera::new(
                Vec3::ZERO,
                Vec3::Z,
                PI * 0.5,
                constants.z_near,
                constants.z_far,
            );

            App {
                scene_program,
                depth_program,
                ssrt_program,
                rc_program,
                post_pass_program,
                objects,
                scene,
                cascades,
                screen_width,
                screen_height,
                constants_ssbo,
                constants,
                debug_cascade_index: 0,
                debug_cascades_merged: true,
                debug: false,
                debug_mode: DebugMode::RadianceCascades,
                debug_mode_idx: 0,
                mouse_is_down: false,
                voxelizer,
                quad_renderer,
                camera,
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        let t_start = elapsed_time();
        self.draw_scene(gl);
        self.generate_hi_z_buffer(gl);
        self.calculate_cascades(gl);
        self.voxelizer.voxelize(gl, &self.objects);
        if self.debug {
            match self.debug_mode {
                DebugMode::RayMarching => {
                    self.draw_ssrt(gl);
                }
                DebugMode::RadianceCascades => unsafe {
                    let cascade_res = Vec2::new(
                        self.constants.c0_resolution.x,
                        self.constants.c0_resolution.y / 2.0_f32.powi(self.debug_cascade_index),
                    );
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.cascades.fb));
                    gl.read_buffer(COLOR_ATTACHMENT0);
                    gl.framebuffer_texture(
                        READ_FRAMEBUFFER,
                        COLOR_ATTACHMENT0,
                        Some(self.cascades.cascades[self.debug_cascade_index as usize]),
                        0,
                    );
                    gl.viewport(0, 0, self.screen_width, self.screen_height);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.blit_framebuffer(
                        0,
                        0,
                        cascade_res.x as _,
                        cascade_res.y as _,
                        0,
                        0,
                        self.screen_width,
                        self.screen_height,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                },
                DebugMode::DepthBuffer => unsafe {
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
                    gl.read_buffer(COLOR_ATTACHMENT3);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.viewport(0, 0, self.screen_width, self.screen_height);
                    gl.blit_framebuffer(
                        0,
                        0,
                        self.screen_width,
                        self.screen_height,
                        0,
                        0,
                        self.screen_width,
                        self.screen_height,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                },
                DebugMode::Scene => unsafe {
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
                    gl.read_buffer(COLOR_ATTACHMENT0);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.viewport(0, 0, self.screen_width, self.screen_height);
                    gl.blit_framebuffer(
                        0,
                        0,
                        self.screen_width,
                        self.screen_height,
                        0,
                        0,
                        self.screen_width,
                        self.screen_height,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                },
                DebugMode::Voxel => {
                    //self.voxelizer.visualize(
                    //    gl,
                    //    &self.quad_renderer,
                    //    &self.camera,
                    //    self.constants.screen_res,
                    //);
                    self.voxelizer
                        .visualize_instanced(gl, &self.camera, self.constants.screen_res);
                }
            }
        } else {
            self.integrate_radiance(gl);
        }
        let t_end = elapsed_time();
        println!("Time to render: {:?}", t_end - t_start);
    }

    fn key_down(
        &mut self,
        keycode: Option<Keycode>,
        scancode: Option<Scancode>,
        keymod: Mod,
        repeat: bool,
    ) {
        if let Some(kc) = keycode {
            let cam_right = self.camera.right();
            let direction = match kc {
                Keycode::W => self.camera.look_direction,
                Keycode::S => -self.camera.look_direction,
                Keycode::A => -cam_right,
                Keycode::D => cam_right,
                Keycode::SPACE => {
                    if keymod == Mod::LSHIFTMOD {
                        -Vec3::Y
                    } else {
                        Vec3::Y
                    }
                }
                _ => Vec3::ZERO,
            };
            self.camera.move_by(direction * delta_time());
        }
    }

    fn mouse_down(&mut self, button: MouseButton, x: i32, y: i32) {
        match button {
            MouseButton::Right => {
                self.mouse_is_down = true;
            }
            _ => {}
        }
    }

    fn mouse_up(&mut self, button: MouseButton, x: i32, y: i32) {
        match button {
            MouseButton::Right => {
                self.mouse_is_down = false;
            }
            _ => {}
        }
    }

    fn mouse_moved_rel(&mut self, xrel: i32, yrel: i32) {
        if self.mouse_is_down {
            let rotation = (Quat::from_rotation_y(2.0 * -xrel as f32 / self.screen_width as f32)
                * Quat::from_rotation_x(2.0 * yrel as f32 / self.screen_height as f32))
            .normalize();
            self.camera.rotate(rotation);
        }
    }

    fn ui(&mut self, gl: &Context, ui: &mut imgui::Ui) {
        let mut constants_changed = false;
        ui.checkbox("Enable debug mode", &mut self.debug);

        let debug_modes = vec![
            "Radiance cascades",
            "Ray marcher",
            "Scene",
            "Depth",
            "Voxel",
        ];
        if ui.combo_simple_string("Debug mode", &mut self.debug_mode_idx, &debug_modes) {
            match debug_modes[self.debug_mode_idx] {
                "Radiance cascades" => {
                    self.debug_mode = DebugMode::RadianceCascades;
                }
                "Ray marcher" => {
                    self.debug_mode = DebugMode::RayMarching;
                }
                "Scene" => {
                    self.debug_mode = DebugMode::Scene;
                }
                "Depth" => {
                    self.debug_mode = DebugMode::DepthBuffer;
                }
                "Voxel" => {
                    self.debug_mode = DebugMode::Voxel;
                }
                _ => unreachable!(),
            }
        }
        if ui.tree_node("Radiance cascades").is_some() {
            constants_changed = constants_changed
                || ui
                    .input_int("Cascade index", &mut self.debug_cascade_index)
                    .build();
            constants_changed = constants_changed
                || ui.slider(
                    "Interval length",
                    0.0,
                    200.0,
                    &mut self.constants.c0_interval_length,
                );
            constants_changed = constants_changed
                || ui.checkbox("Merged cascades", &mut self.debug_cascades_merged);
        }
        if ui.tree_node("Ray marching").is_some() {
            constants_changed = constants_changed
                || ui.slider(
                    "Max ray length",
                    0.0,
                    200.0,
                    &mut self.constants.max_ray_distance,
                );
            constants_changed = constants_changed
                || ui
                    .input_float("Max step count", &mut self.constants.max_steps)
                    .build();
        }

        if constants_changed {
            unsafe {
                gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(self.constants_ssbo));
                gl.buffer_data_u8_slice(
                    SHADER_STORAGE_BUFFER,
                    bytemuck::bytes_of(&self.constants),
                    STATIC_DRAW,
                );
            }
        }
    }
}

fn main() {
    App::sdl2_window("Radiance cascades 3D prototype")
        .gl_version(4, 5)
        .debug_message_callback(debug_message_callback)
        .window_size(1024, 1024)
        .start();
}
