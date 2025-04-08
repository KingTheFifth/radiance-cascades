#[macro_use]
extern crate load_file;

use std::{collections::VecDeque, f32::consts::PI, ffi::CStr};

use bytemuck::{Pod, Zeroable};
use camera::Camera;
use microglut::{
    delta_time, elapsed_time,
    glam::{Mat4, Quat, Vec2, Vec3, Vec4},
    glow::{
        Context, HasContext, NativeBuffer, NativeProgram, PixelPackData, BLEND, COLOR_ATTACHMENT0,
        COLOR_ATTACHMENT3, COLOR_BUFFER_BIT, CULL_FACE, DEBUG_OUTPUT, DEPTH_BUFFER_BIT, DEPTH_TEST,
        DRAW_FRAMEBUFFER, FRAMEBUFFER, LINEAR, MULTISAMPLE, ONE_MINUS_SRC_ALPHA, READ_FRAMEBUFFER,
        RGBA, SHADER_STORAGE_BUFFER, SRC_ALPHA, STATIC_DRAW, TEXTURE0, TEXTURE1, TEXTURE2,
        TEXTURE_2D, TEXTURE_MAX_LEVEL, UNSIGNED_BYTE,
    },
    imgui, load_shaders, load_tangent_buf,
    sdl2::{
        keyboard::{Keycode, Mod, Scancode},
        mouse::MouseButton,
    },
    MaterialBindings, MicroGLUT, Model, Texture, Window,
};
use object::Object;
use quad_renderer::QuadRenderer;
use radiance_cascades::RadianceCascades;
use scene_fbo::SceneFBO;
use strum::{Display, VariantArray};
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
mod object;
mod quad_renderer;
mod radiance_cascades;
mod scene_fbo;
mod voxelizer;

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
struct SceneMatrices {
    world_to_view: Mat4,
    world_to_view_inv: Mat4,
    perspective: Mat4,
    perspective_inv: Mat4,
    screen_resolution: Vec2,
    screen_resolution_inv: Vec2,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
struct HiZConstants {
    /// Hi Z screen-space ray marching
    pub hi_z_resolution: Vec2,
    pub inv_hi_z_resolution: Vec2,

    pub hi_z_start_mip_level: f32,
    pub hi_z_max_mip_level: f32,
    pub max_steps: f32,
    pub max_ray_distance: f32,

    pub z_far: f32,
    pub z_near: f32,
    _padding: [f32; 2],
}

#[derive(Display, VariantArray, PartialEq, Clone, Copy)]
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

    screen_resolution: Vec2,
    objects: Vec<Object>,
    camera: Camera,

    scene: SceneFBO,
    scene_matrices: SceneMatrices,
    scene_matrices_ssbo: NativeBuffer,
    scene_matrices_binding: u32,

    hi_z_constants: HiZConstants,
    hi_z_constants_ssbo: NativeBuffer,
    hi_z_constants_binding: u32,

    quad_renderer: QuadRenderer,
    radiance_cascades: RadianceCascades,
    voxelizer: Voxelizer,

    debug: bool,
    debug_mode: DebugMode,

    mouse_is_down: bool,
    frame_times: VecDeque<f32>,
}

impl App {
    fn draw_scene(&mut self, gl: &Context) {
        unsafe {
            let w_t_v = self.camera.view_transform();
            let perspective_mat = self.camera.perspective_transform();

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(self.scene_matrices_ssbo));
            gl.buffer_sub_data_u8_slice(SHADER_STORAGE_BUFFER, 0x00, bytemuck::bytes_of(&w_t_v));
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0x40,
                bytemuck::bytes_of(&w_t_v.inverse()),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0x80,
                bytemuck::bytes_of(&perspective_mat),
            );
            gl.buffer_sub_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                0xC0,
                bytemuck::bytes_of(&perspective_mat.inverse()),
            );
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

            gl.bind_framebuffer(FRAMEBUFFER, Some(self.scene.fb));
            gl.use_program(Some(self.scene_program));
            gl.enable(BLEND);
            gl.enable(DEPTH_TEST);
            //gl.enable(CULL_FACE);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);

            gl.viewport(
                0,
                0,
                self.screen_resolution.x as _,
                self.screen_resolution.y as _,
            );
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

            let material_bindings = MaterialBindings {
                ambient: None,
                emissive: Some(String::from("emissive")),
                diffuse: Some(String::from("diffuse")),
                specular: Some(String::from("specular")),
                shininess: None,
                dissolve: Some(String::from("opacity")),
                optical_density: None,
                ambient_texture: None,
                diffuse_texture: Some((String::from("diffuse_tex"), 0)),
                specular_texture: Some((String::from("specular_tex"), 1)),
                normal_texture: Some((String::from("normal_map"), 2)),
                shininess_texture: None,
                dissolve_texture: Some((String::from("opacity_tex"), 3)),
                illumination_model: None,
            };

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
                object.model.draw(
                    gl,
                    self.scene_program,
                    "position",
                    Some("v_normal"),
                    Some("v_tex_coord"),
                    Some("v_tangent"),
                    Some("v_bitangent"),
                    Some(&material_bindings),
                );
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.disable(BLEND);
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);
        }
    }

    fn generate_hi_z_buffer(&self, gl: &Context) {
        let start_dims = self.hi_z_constants.hi_z_resolution;
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
            for level in 1..(self.hi_z_constants.hi_z_max_mip_level as i32) + 1 {
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
            gl.viewport(
                0,
                0,
                self.screen_resolution.x as _,
                self.screen_resolution.y as _,
            );
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

            gl.viewport(
                0,
                0,
                self.screen_resolution.x as _,
                self.screen_resolution.y as _,
            );
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            self.quad_renderer.draw_screen_quad(gl, self.ssrt_program);
        }
    }

    fn save_screen_to(&self, gl: &Context) {
        unsafe {
            let mut image: Vec<u8> = vec![];
            image.resize(4 * 1280 * 720, 0);
            let p = PixelPackData::Slice(&mut image);
            //gl.read_buffer(COLOR_ATTACHMENT0);
            //gl.pixel_store_i32(parameter, value);
            gl.read_pixels(
                0,
                0,
                self.screen_resolution.x as _,
                self.screen_resolution.y as _,
                RGBA,
                UNSIGNED_BYTE,
                p,
            );

            let rev_img: Vec<u8> = image
                .chunks(4 * 1280)
                .into_iter()
                .rev()
                .flatten()
                .copied()
                .collect();

            image::save_buffer(
                "test.png",
                &rev_img,
                self.screen_resolution.x as _,
                self.screen_resolution.y as _,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

impl MicroGLUT for App {
    fn init(gl: &Context, window: &Window) -> Self {
        let screen_width = window.size().0 as i32;
        let screen_height = window.size().1 as i32;
        let screen_resolution = Vec2::new(screen_width as _, screen_height as _);
        let screen_resolution_inv =
            Vec2::new(1.0 / screen_width as f32, 1.0 / screen_height as f32);

        let quad_renderer = QuadRenderer::new(gl);

        let scene = SceneFBO::init(gl, screen_width, screen_height);

        let voxel_res = 256.0;
        let voxel_origin = Vec3::new(0.0, 0.0, 0.0);
        let voxel_volume_half_side = 6.0;
        let voxelizer = Voxelizer::new(
            gl,
            Vec3::new(voxel_res, voxel_res, voxel_res),
            voxel_origin,
            voxel_volume_half_side,
        );
        voxelizer.clear_voxels(gl, &quad_renderer, Vec4::new(0., 0., 0., 0.0));

        let camera = Camera::new(
            Vec3::new(0., 1., -1.),
            Vec3::Z,
            PI * 0.25,
            0.3,
            30.0,
            screen_width as f32 / screen_height as f32,
        );

        let scene_matrices_binding = 0;
        let hi_z_constants_binding = 1;
        let rc_binding = 2;
        let radiance_cascades = RadianceCascades::new(
            gl,
            6.0,
            screen_resolution,
            4.0,
            rc_binding,
            scene_matrices_binding,
            hi_z_constants_binding,
        );

        let scene_matrices = SceneMatrices {
            world_to_view: camera.view_transform(),
            world_to_view_inv: camera.view_transform().inverse(),
            perspective: camera.perspective_transform(),
            perspective_inv: camera.perspective_transform().inverse(),
            screen_resolution,
            screen_resolution_inv,
        };

        let hi_z_constants = HiZConstants {
            hi_z_resolution: screen_resolution,
            inv_hi_z_resolution: screen_resolution_inv,
            hi_z_start_mip_level: 0.0,
            hi_z_max_mip_level: 10.0,
            max_steps: 400.0,
            max_ray_distance: 30.0,
            z_near: 0.3,
            z_far: 30.0,
            _padding: [0.0, 0.0],
        };

        unsafe {
            gl.enable(MULTISAMPLE);

            // Create and bind shader storage buffer objects
            let scene_matrices_ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(scene_matrices_ssbo));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::bytes_of(&scene_matrices),
                STATIC_DRAW,
            );
            gl.bind_buffer_base(
                SHADER_STORAGE_BUFFER,
                scene_matrices_binding,
                Some(scene_matrices_ssbo),
            );

            let hi_z_constants_ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(hi_z_constants_ssbo));
            gl.buffer_data_u8_slice(
                SHADER_STORAGE_BUFFER,
                bytemuck::bytes_of(&hi_z_constants),
                STATIC_DRAW,
            );
            gl.bind_buffer_base(
                SHADER_STORAGE_BUFFER,
                hi_z_constants_binding,
                Some(hi_z_constants_ssbo),
            );
            gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

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

            let scene_matrices_ssbo_loc = gl
                .get_shader_storage_block_index(ssrt_program, "SceneMatrices")
                .unwrap();

            gl.shader_storage_block_binding(
                ssrt_program,
                scene_matrices_ssbo_loc,
                scene_matrices_binding,
            );
            let constants_ssbo_loc = gl
                .get_shader_storage_block_index(ssrt_program, "HiZConstants")
                .unwrap();
            gl.shader_storage_block_binding(
                ssrt_program,
                constants_ssbo_loc,
                hi_z_constants_binding,
            );

            gl.viewport(0, 0, screen_width, screen_height);

            let rock = Model::load_obj_data(
                gl,
                include_bytes!("../models/Rock.obj"),
                Some(&|_| tobj::load_mtl_buf(&mut &include_bytes!("../models/Rock.mtl")[..])),
                None,
                None,
                false,
            );

            let cube_model = Model::load_obj_data(
                gl,
                include_bytes!("../models/cube.obj"),
                None,
                None,
                None,
                false,
            );
            let cube = Object::new(cube_model);

            let suzanne_model = Model::load_obj_data(
                gl,
                include_bytes!("../models/suzanne.obj"),
                None,
                None,
                None,
                false,
            );
            let suzanne = Object::new(suzanne_model);

            //let armadillo_model =
            //    Model::load_obj_data(gl, include_bytes!("../models/armadillo.obj"), None, None);
            //let armadillo = Object::new(armadillo_model);

            let sphere_model = Model::load_obj_data(
                gl,
                include_bytes!("../models/groundsphere.obj"),
                None,
                None,
                None,
                false,
            );
            let sphere = Object::new(sphere_model);

            let sponza_model = Model::load_obj_data(
                gl,
                include_bytes!("../models/sponza.obj"),
                Some(&|_| tobj::load_mtl_buf(&mut &include_bytes!("../models/sponza.mtl")[..])),
                Some(&|name| {
                    load_bytes!(&format!("../textures/sponza_textures/{}", name)).to_vec()
                }),
                Some(&|name| {
                    load_bytes!(&format!(
                        "../models/sponza_tangents/g {}_tangents.txt",
                        name
                    ))
                    .to_vec()
                }),
                false,
            );

            let (vase_tangents, vase_bitangents) = load_tangent_buf(load_bytes!(
                "../models/sponza_tangents/vase_round_tangents.txt"
            ))
            .unwrap();

            sponza_model.meshes.iter().for_each(|mesh| {
                if mesh.material == Some(1) {
                    mesh.load_tangents(gl, &vase_tangents, &vase_bitangents);
                }
            });
            let sponza = Object::new(sponza_model);

            //let objects = vec![
            //    Object::new(rock.clone())
            //        .with_rotation(Quat::from_rotation_x(-0.2))
            //        .with_translation(Vec3::new(0.0, 0.0, 2.0))
            //        .with_albedo(Vec4::new(0.0, 0.0, 0.0, 1.0))
            //        .with_emissive(Vec4::new(4.0, 4.0, 4.0, 1.0)),
            //    Object::new(rock.clone())
            //        .with_rotation(Quat::from_rotation_x(-1.0))
            //        .with_translation(Vec3::new(0.5, 0.0, 1.0))
            //        .with_albedo(Vec4::new(0.0, 0.5, 0.8, 1.0)),
            //    Object::new(rock.clone())
            //        .with_rotation(Quat::from_rotation_x(-PI))
            //        .with_uniform_scale(15.0)
            //        .with_translation(Vec3::new(0.0, -0.5, 3.0)),
            //    Object::new(rock.clone())
            //        .with_rotation(Quat::from_rotation_x(-3.0 * PI / 2.0))
            //        .with_uniform_scale(15.0)
            //        .with_translation(Vec3::new(0.0, 0.0, 6.0))
            //        .with_albedo(Vec4::new(0.5, 0.1, 0.5, 1.0)),
            //];

            let objects = vec![
                //cube.clone()
                //    .with_emissive(Vec4::new(1.0, 1.0, 1.0, 1.0))
                //    .with_albedo(Vec4::W)
                //    .with_translation(Vec3::new(0., 1.1, 2.)),
                //cube.clone()
                //    .with_albedo(Vec4::new(0.05, 0.1, 1., 1.))
                //    .with_scale(Vec3::new(10., 0.25, 10.))
                //    .with_translation(Vec3::new(0., -0.25, 0.)), //.with_emissive(Vec4::new(0.0, 0.0, 0.3, 1.0)),
                //suzanne
                //    .clone()
                //    .with_albedo(Vec4::ONE)
                //    .with_rotation(Quat::from_rotation_y(-PI * 0.25))
                //    .with_translation(Vec3::new(6.0, -0.2, -2.0)),
                sponza.with_uniform_scale(0.01),
            ];

            App {
                scene_program,
                depth_program,
                ssrt_program,
                objects,
                scene,
                screen_resolution,
                scene_matrices,
                scene_matrices_ssbo,
                scene_matrices_binding,
                hi_z_constants,
                hi_z_constants_ssbo,
                hi_z_constants_binding,
                debug: false,
                debug_mode: DebugMode::RadianceCascades,
                mouse_is_down: false,
                voxelizer,
                quad_renderer,
                camera,
                radiance_cascades,
                frame_times: VecDeque::new(),
            }
        }
    }

    fn display(&mut self, gl: &Context, window: &Window) {
        let t_start = elapsed_time();
        self.draw_scene(gl);
        self.generate_hi_z_buffer(gl);
        self.voxelizer
            .clear_voxels(gl, &self.quad_renderer, Vec4::new(0.0, 0.0, 0.0, 0.0));
        self.voxelizer.voxelize(gl, &self.objects);
        if self.debug {
            match self.debug_mode {
                DebugMode::RayMarching => {
                    self.draw_ssrt(gl);
                }
                DebugMode::RadianceCascades => {
                    self.radiance_cascades.render_debug(
                        gl,
                        self.screen_resolution,
                        &self.scene,
                        &self.voxelizer,
                    );
                }
                DebugMode::DepthBuffer => unsafe {
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
                    gl.read_buffer(COLOR_ATTACHMENT3);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.viewport(
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.y as _,
                    );
                    gl.blit_framebuffer(
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.x as _,
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.y as _,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                },
                DebugMode::Scene => unsafe {
                    gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.scene.fb));
                    gl.read_buffer(COLOR_ATTACHMENT0);
                    gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
                    gl.viewport(
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.y as _,
                    );
                    gl.blit_framebuffer(
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.y as _,
                        0,
                        0,
                        self.screen_resolution.x as _,
                        self.screen_resolution.y as _,
                        COLOR_BUFFER_BIT,
                        LINEAR as _,
                    );
                },
                DebugMode::Voxel => {
                    self.voxelizer.visualize(
                        gl,
                        &self.camera,
                        self.screen_resolution,
                        &self.quad_renderer,
                    );
                }
            }
        } else {
            self.radiance_cascades
                .render(gl, self.screen_resolution, &self.scene, &self.voxelizer);
        }
        let t_end = elapsed_time();
        self.frame_times.push_back(t_end - t_start);
        if self.frame_times.len() > 100 {
            self.frame_times.rotate_left(self.frame_times.len() - 100);
            self.frame_times.truncate(100);
        }
        //println!("Time to render: {:?}", t_end - t_start);
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
            self.camera
                .move_by(direction * delta_time() * self.camera.walk_speed);
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
            let speed = self.camera.rotational_speed;
            let rotation = (Quat::from_rotation_y(speed * -xrel as f32 / self.screen_resolution.x)
                * Quat::from_rotation_x(speed * yrel as f32 / self.screen_resolution.y))
            .normalize();
            self.camera.rotate(rotation);
        }
    }

    fn ui(&mut self, gl: &Context, ui: &mut imgui::Ui) {
        let mut constants_changed = false;
        ui.checkbox("Enable debug mode", &mut self.debug);

        if ui.button("Save screenshot") {
            self.save_screen_to(gl);
        }

        if let Some(cb) = ui.begin_combo("Debug mode", self.debug_mode.to_string()) {
            for cur in DebugMode::VARIANTS {
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

        self.camera.ui(ui);
        self.radiance_cascades.ui(gl, ui);
        self.voxelizer.ui(ui);

        if ui.tree_node("Ray marching").is_some() {
            constants_changed = constants_changed
                || ui.slider(
                    "Max ray length",
                    0.0,
                    200.0,
                    &mut self.hi_z_constants.max_ray_distance,
                );
            constants_changed = constants_changed
                || ui
                    .input_float("Max step count", &mut self.hi_z_constants.max_steps)
                    .build();
        }

        if constants_changed {
            unsafe {
                gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(self.hi_z_constants_ssbo));
                gl.buffer_data_u8_slice(
                    SHADER_STORAGE_BUFFER,
                    bytemuck::bytes_of(&self.hi_z_constants),
                    STATIC_DRAW,
                );
            }
        }

        let fps = self.frame_times.len() as f32 / self.frame_times.iter().sum::<f32>();
        ui.plot_lines("Frame times", self.frame_times.make_contiguous())
            .overlay_text(format!("FPS {}", fps))
            .build();
    }
}

fn main() {
    App::sdl2_window("Radiance cascades 3D prototype")
        .gl_version(4, 5)
        .debug_message_callback(debug_message_callback)
        .window_size(1280, 720)
        .start();
}
