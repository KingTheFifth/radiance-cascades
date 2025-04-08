use bytemuck::{Pod, Zeroable};
use cascade_fbo::CascadeFBO;
use microglut::{
    glam::Vec2,
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, COLOR_ATTACHMENT0, COLOR_BUFFER_BIT,
        DRAW_FRAMEBUFFER, FRAMEBUFFER, LINEAR, READ_FRAMEBUFFER, READ_ONLY, RGBA16F,
        SHADER_STORAGE_BUFFER, STATIC_DRAW, TEXTURE0, TEXTURE1, TEXTURE2, TEXTURE3, TEXTURE4,
        TEXTURE_2D,
    },
    imgui, LoadShaders,
};
use strum::{Display, VariantArray};

use crate::{quad_renderer::QuadRenderer, scene_fbo::SceneFBO, voxelizer::Voxelizer};

mod cascade_fbo;

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
struct RadianceCascadesConstants {
    c0_resolution: Vec2,
    cascade_count: f32,
    c0_probe_spacing: f32,
    c0_interval_length: f32,
    normal_offset: f32,
    gamma: f32,
    ambient_occlusion_factor: f32,
    diffuse_intensity: f32,
    ambient_occlusion: f32,
    _padding: [f32; 2],
}

#[derive(Display, VariantArray, PartialEq, Clone, Copy)]
enum DebugModes {
    Cascades,
    Upscale,
}

pub struct RadianceCascades {
    cascade_program: NativeProgram,
    integration_program: NativeProgram,
    upscale_program: NativeProgram,
    cascades: CascadeFBO,
    quad_renderer: QuadRenderer,

    constants: RadianceCascadesConstants,
    constants_ssbo: NativeBuffer,
    constants_ssbo_binding: u32,
    ambient_level: f32,

    // Debug info
    merge_cascades: bool,
    debug_cascade_index: usize,
    debug_mode: DebugModes,
}

impl RadianceCascadesConstants {
    pub fn create_shader_storage_buffer(&self, gl: &Context, binding_point: u32) -> NativeBuffer {
        unsafe {
            let ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(ssbo));
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, binding_point, Some(ssbo));
            self.upload_to_buffer(gl, ssbo);
            ssbo
        }
    }

    pub fn upload_to_buffer(&self, gl: &Context, shader_storage_buffer: NativeBuffer) {
        unsafe {
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(shader_storage_buffer));
            gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, bytemuck::bytes_of(self), STATIC_DRAW);
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);
        }
    }
}

impl RadianceCascades {
    pub fn new(
        gl: &Context,
        cascade_count: f32,
        screen_resolution: Vec2,
        probe_spacing: f32,
        binding_point: u32,
        scene_matrices_binding: u32,
        hi_z_constants_binding: u32,
    ) -> Self {
        let interval_length = Vec2::ZERO.distance(Vec2::new(probe_spacing, probe_spacing)) * 0.5;
        let probe_spacing_adjusted = ceil_to_power_of_n(probe_spacing, 2.0);
        let interval_length_adjusted = ceil_to_multiple_of_n(interval_length, 2.0);
        let cascade_width = 4.0 * screen_resolution.x / probe_spacing_adjusted;
        let cascade_height = 4.0 * screen_resolution.y / probe_spacing_adjusted;
        let c0_resolution = Vec2::new(cascade_width, cascade_height);
        //let num_cascades = Vec2::ZERO.distance(screen_dims).log(4.0).ceil();

        let cascades = CascadeFBO::new(gl, c0_resolution, cascade_count as _);

        let cascade_program =
            LoadShaders::new(include_str!("rc.vert"), include_str!("rc.frag")).compile(gl);
        let integration_program =
            LoadShaders::new(include_str!("rc.vert"), include_str!("integrate.frag")).compile(gl);
        let upscale_program =
            LoadShaders::new(include_str!("rc.vert"), include_str!("upscale.frag")).compile(gl);

        let quad_renderer = QuadRenderer::new(gl);

        let constants = RadianceCascadesConstants {
            c0_interval_length: interval_length_adjusted,
            c0_probe_spacing: probe_spacing_adjusted,
            c0_resolution,
            cascade_count,
            normal_offset: 0.1,
            ambient_occlusion_factor: 2.0,
            gamma: 1.0,
            diffuse_intensity: 15.0,
            ambient_occlusion: 1.0,
            _padding: [0.0, 0.0],
        };
        let constants_ssbo_binding = binding_point;
        let constants_ssbo = constants.create_shader_storage_buffer(gl, constants_ssbo_binding);
        constants.upload_to_buffer(gl, constants_ssbo);

        unsafe {
            let mut hi_z_constants_ssbo_loc = gl
                .get_shader_storage_block_index(cascade_program, "HiZConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                cascade_program,
                hi_z_constants_ssbo_loc,
                hi_z_constants_binding,
            );

            let mut scene_matrices_ssbo_loc = gl
                .get_shader_storage_block_index(cascade_program, "SceneMatrices")
                .unwrap();
            gl.shader_storage_block_binding(
                cascade_program,
                scene_matrices_ssbo_loc,
                scene_matrices_binding,
            );

            let mut constants_ssbo_loc = gl
                .get_shader_storage_block_index(cascade_program, "RCConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                cascade_program,
                constants_ssbo_loc,
                constants_ssbo_binding,
            );

            hi_z_constants_ssbo_loc = gl
                .get_shader_storage_block_index(integration_program, "HiZConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                integration_program,
                hi_z_constants_ssbo_loc,
                hi_z_constants_binding,
            );
            constants_ssbo_loc = gl
                .get_shader_storage_block_index(integration_program, "RCConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                integration_program,
                constants_ssbo_loc,
                constants_ssbo_binding,
            );
            scene_matrices_ssbo_loc = gl
                .get_shader_storage_block_index(integration_program, "SceneMatrices")
                .unwrap();
            gl.shader_storage_block_binding(
                integration_program,
                scene_matrices_ssbo_loc,
                scene_matrices_binding,
            );

            hi_z_constants_ssbo_loc = gl
                .get_shader_storage_block_index(upscale_program, "HiZConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                upscale_program,
                hi_z_constants_ssbo_loc,
                hi_z_constants_binding,
            );
        }

        Self {
            cascade_program,
            integration_program,
            upscale_program,
            cascades,
            quad_renderer,
            constants,
            constants_ssbo,
            constants_ssbo_binding,
            merge_cascades: true,
            debug_cascade_index: 0,
            ambient_level: 0.1,
            debug_mode: DebugModes::Cascades,
        }
    }

    fn calculate_cascades(
        &mut self,
        gl: &Context,
        screen_resolution: Vec2,
        scene: &SceneFBO,
        voxelizer: &Voxelizer,
    ) {
        unsafe {
            gl.use_program(Some(self.cascade_program));
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.cascades.fb));
            gl.active_texture(TEXTURE1);
            gl.bind_texture(TEXTURE_2D, Some(scene.albedo));
            gl.active_texture(TEXTURE2);
            gl.bind_texture(TEXTURE_2D, Some(scene.emissive));
            gl.active_texture(TEXTURE3);
            gl.bind_texture(TEXTURE_2D, Some(scene.normal));
            gl.active_texture(TEXTURE4);
            gl.bind_texture(TEXTURE_2D, Some(scene.hi_z_texture));

            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "prev_cascade")
                    .as_ref(),
                0,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "scene_albedo")
                    .as_ref(),
                1,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "scene_emissive")
                    .as_ref(),
                2,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "scene_normal")
                    .as_ref(),
                3,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "hi_z_tex")
                    .as_ref(),
                4,
            );

            gl.uniform_1_i32(
                gl.get_uniform_location(self.cascade_program, "merge_cascades")
                    .as_ref(),
                self.merge_cascades.into(),
            );

            gl.bind_image_texture(
                0,
                voxelizer.voxel_texture(),
                0,
                false,
                0,
                READ_ONLY,
                RGBA16F,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.cascade_program, "step_count")
                    .as_ref(),
                voxelizer.step_count(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.cascade_program, "world_to_voxel")
                    .as_ref(),
                false,
                voxelizer.world_to_voxel().as_ref(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.cascade_program, "voxel_resolution")
                    .as_ref(),
                voxelizer.resolution().as_ref(),
            );

            for n in (0..self.constants.cascade_count as i32).rev() {
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.cascade_program, "cascade_index")
                        .as_ref(),
                    n as _,
                );

                gl.active_texture(TEXTURE0);
                self.cascades.bind_cascade_as_texture(
                    gl,
                    (n + 1).min(self.constants.cascade_count as i32 - 1) as _,
                    TEXTURE0,
                );

                gl.viewport(
                    0,
                    0,
                    self.constants.c0_resolution.x as _,
                    self.constants.c0_resolution.y as _,
                );
                self.cascades.bind_cascade_as_output(gl, n as _);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(COLOR_BUFFER_BIT);
                self.quad_renderer
                    .draw_screen_quad(gl, self.cascade_program);
            }

            gl.viewport(0, 0, screen_resolution.x as _, screen_resolution.y as _);
            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    fn integrate_radiance(
        &self,
        gl: &Context,
        cascade_index: usize,
        screen_resolution: Vec2,
        scene: &SceneFBO,
    ) {
        unsafe {
            gl.use_program(Some(self.integration_program));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.integration_program, "cascade")
                    .as_ref(),
                0,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.integration_program, "scene_normal")
                    .as_ref(),
                1,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.integration_program, "scene_albedo")
                    .as_ref(),
                2,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.integration_program, "scene_emissive")
                    .as_ref(),
                3,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.integration_program, "depth_tex")
                    .as_ref(),
                4,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.integration_program, "cascade_index")
                    .as_ref(),
                cascade_index as _,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.integration_program, "ambient")
                    .as_ref(),
                self.ambient_level,
            );

            self.cascades
                .bind_cascade_as_texture(gl, cascade_index, TEXTURE0);
            gl.active_texture(TEXTURE1);
            gl.bind_texture(TEXTURE_2D, Some(scene.normal));
            gl.active_texture(TEXTURE2);
            gl.bind_texture(TEXTURE_2D, Some(scene.albedo));
            gl.active_texture(TEXTURE3);
            gl.bind_texture(TEXTURE_2D, Some(scene.emissive));
            gl.active_texture(TEXTURE4);
            gl.bind_texture(TEXTURE4, Some(scene.hi_z_texture));

            gl.viewport(0, 0, screen_resolution.x as _, screen_resolution.y as _);
            gl.clear(COLOR_BUFFER_BIT);
            self.quad_renderer
                .draw_screen_quad(gl, self.integration_program);
        }
    }

    pub fn render_debug(
        &mut self,
        gl: &Context,
        screen_resolution: Vec2,
        scene: &SceneFBO,
        voxelizer: &Voxelizer,
    ) {
        let screen_width = screen_resolution.x as i32;
        let screen_height = screen_resolution.y as i32;
        match self.debug_mode {
            DebugModes::Cascades => {
                self.calculate_cascades(gl, screen_resolution, scene, voxelizer);

                let cascade_width = self.constants.c0_resolution.x as i32;
                let cascade_height = self.constants.c0_resolution.y as i32;
                unsafe {
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.cascades.fb));
                    gl.read_buffer(COLOR_ATTACHMENT0);
                    gl.framebuffer_texture(
                        READ_FRAMEBUFFER,
                        COLOR_ATTACHMENT0,
                        Some(self.cascades.cascades[self.debug_cascade_index]),
                        0,
                    );
                    gl.viewport(0, 0, screen_width, screen_height);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.blit_framebuffer(
                        0,
                        0,
                        cascade_width,
                        cascade_height,
                        0,
                        0,
                        screen_width,
                        screen_height,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                }
            }
            DebugModes::Upscale => unsafe {
                gl.bind_framebuffer(FRAMEBUFFER, None);
                gl.viewport(0, 0, screen_width, screen_height);
                gl.clear(COLOR_BUFFER_BIT);
                gl.use_program(Some(self.upscale_program));

                gl.uniform_1_i32(
                    gl.get_uniform_location(self.upscale_program, "full_res_tex")
                        .as_ref(),
                    0,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.upscale_program, "depth_tex")
                        .as_ref(),
                    1,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.upscale_program, "normal_tex")
                        .as_ref(),
                    2,
                );

                gl.active_texture(TEXTURE0);
                gl.bind_texture(TEXTURE_2D, Some(scene.albedo));

                gl.active_texture(TEXTURE1);
                gl.bind_texture(TEXTURE_2D, Some(scene.depth_texture));

                gl.active_texture(TEXTURE2);
                gl.bind_texture(TEXTURE_2D, Some(scene.normal));

                self.quad_renderer
                    .draw_screen_quad(gl, self.upscale_program);
            },
        }
    }

    pub fn render(
        &mut self,
        gl: &Context,
        screen_resolution: Vec2,
        scene: &SceneFBO,
        voxelizer: &Voxelizer,
    ) {
        self.calculate_cascades(gl, screen_resolution, scene, voxelizer);
        self.integrate_radiance(gl, self.debug_cascade_index, screen_resolution, scene);
    }

    pub fn ui(&mut self, gl: &Context, ui: &imgui::Ui) {
        let mut constants_changed = false;
        let mut ao = self.constants.ambient_occlusion != 0.0;
        if ui.tree_node("Radiance cascades").is_some() {
            if ui.tree_node("GI parameters").is_some() {
                constants_changed = constants_changed || ui.checkbox("AO", &mut ao);
                constants_changed = constants_changed
                    || ui
                        .input_float("AO factor", &mut self.constants.ambient_occlusion_factor)
                        .build();
                constants_changed = constants_changed
                    || ui
                        .input_float("Diffuse intensity", &mut self.constants.diffuse_intensity)
                        .build();
                constants_changed =
                    constants_changed || ui.input_float("Gamma", &mut self.constants.gamma).build();
            }

            if let Some(cb) = ui.begin_combo("Mode##xx", self.debug_mode.to_string()) {
                for cur in DebugModes::VARIANTS {
                    if &self.debug_mode == cur {
                        ui.set_item_default_focus();
                    }

                    let clicked = ui
                        .selectable_config(cur.to_string())
                        .selected(&self.debug_mode == cur)
                        .build();
                    if clicked {
                        self.debug_mode = *cur;
                    }
                }
                cb.end();
            }

            ui.input_scalar("Cascade index", &mut self.debug_cascade_index)
                .build();

            constants_changed =
                constants_changed || ui.checkbox("Merged cascades", &mut self.merge_cascades);

            constants_changed = constants_changed
                || ui.slider(
                    "Interval length",
                    0.0,
                    10.0,
                    &mut self.constants.c0_interval_length,
                );

            constants_changed = constants_changed
                || ui.slider("Normal offset", 0.0, 1.0, &mut self.constants.normal_offset);

            ui.slider("Ambient level", 0.0, 1.0, &mut self.ambient_level);
        }

        self.constants.ambient_occlusion = ao as i32 as f32;

        if constants_changed {
            self.constants.upload_to_buffer(gl, self.constants_ssbo);
        }
    }
}
