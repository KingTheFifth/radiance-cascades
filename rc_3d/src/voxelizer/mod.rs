use std::f32::consts::PI;

use microglut::{
    glam::{Mat4, Vec2, Vec3},
    glow::{
        Context, HasContext, NativeBuffer, NativeFramebuffer, NativeProgram, NativeTexture,
        NativeVertexArray, ARRAY_BUFFER, BLEND, CLAMP_TO_EDGE, COLOR_ATTACHMENT0, COLOR_BUFFER_BIT,
        CULL_FACE, DEPTH_ATTACHMENT, DEPTH_BUFFER_BIT, DEPTH_COMPONENT16, DEPTH_TEST,
        DRAW_FRAMEBUFFER, ELEMENT_ARRAY_BUFFER, FLOAT, FRAMEBUFFER, LINEAR, READ_BUFFER,
        READ_FRAMEBUFFER, RENDERBUFFER, RGBA, RGBA16F, RGBA8, STATIC_DRAW, TEXTURE0,
        TEXTURE_2D_MULTISAMPLE, TEXTURE_3D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_R,
        TEXTURE_WRAP_S, TEXTURE_WRAP_T, TRIANGLES, UNSIGNED_BYTE, UNSIGNED_INT, WRITE_ONLY,
    },
    LoadShaders,
};

use crate::{camera::Camera, object::Object, quad_renderer::QuadRenderer};

pub struct Voxelizer {
    resolution: Vec3,
    voxel_texture: NativeTexture,
    voxelizer_program: NativeProgram,
    visualizing_program: NativeProgram,
    instanced_visualizing_program: NativeProgram,
    cube_renderer: CubeRenderer,

    // An MSAA render target is needed for an approximation of conservative rasterization
    msaa_fbo: NativeFramebuffer,
}

impl Voxelizer {
    pub fn new(gl: &Context, resolution: Vec3) -> Self {
        unsafe {
            let voxelizer_program =
                LoadShaders::new(include_str!("voxelize.vert"), include_str!("voxelize.frag"))
                    .geometry(include_str!("voxelize.geom"))
                    .compile(gl);

            let visualizing_program = LoadShaders::new(
                include_str!("../vertex.glsl"),
                include_str!("trace_voxels.frag"),
            )
            .compile(gl);
            let instanced_visualizing_program = LoadShaders::new(
                include_str!("voxel_instanced.vert"),
                include_str!("voxel_instanced.frag"),
            )
            .compile(gl);

            let voxel_texture = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_3D, Some(voxel_texture));
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_R, CLAMP_TO_EDGE as _);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as _);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as _);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MAG_FILTER, LINEAR as _);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MIN_FILTER, LINEAR as _);
            gl.tex_image_3d(
                TEXTURE_3D,
                0,
                RGBA16F as _,
                resolution.x as _,
                resolution.y as _,
                resolution.z as _,
                0,
                RGBA,
                UNSIGNED_BYTE,
                None,
            );
            gl.bind_texture(TEXTURE_3D, None);

            let msaa_fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(msaa_fbo));

            let msaa_tex = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D_MULTISAMPLE, Some(msaa_tex));
            gl.tex_storage_2d_multisample(
                TEXTURE_2D_MULTISAMPLE,
                8,
                RGBA8,
                resolution.x as _,
                resolution.y as _,
                true,
            );
            gl.framebuffer_texture(FRAMEBUFFER, COLOR_ATTACHMENT0, Some(msaa_tex), 0);
            gl.bind_texture(TEXTURE_2D_MULTISAMPLE, None);

            let msaa_rb = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(RENDERBUFFER, Some(msaa_rb));
            gl.renderbuffer_storage_multisample(
                RENDERBUFFER,
                8,
                DEPTH_COMPONENT16,
                resolution.x as _,
                resolution.y as _,
            );
            gl.framebuffer_renderbuffer(FRAMEBUFFER, DEPTH_ATTACHMENT, RENDERBUFFER, Some(msaa_rb));
            gl.bind_renderbuffer(RENDERBUFFER, None);

            gl.bind_framebuffer(FRAMEBUFFER, None);

            let cube_renderer = CubeRenderer::new(gl);

            Self {
                resolution,
                voxel_texture,
                voxelizer_program,
                visualizing_program,
                instanced_visualizing_program,
                msaa_fbo,
                cube_renderer,
            }
        }
    }

    pub fn voxelize(&self, gl: &Context, objects: &Vec<Object>) {
        unsafe {
            gl.use_program(Some(self.voxelizer_program));
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.msaa_fbo));
            gl.viewport(0, 0, self.resolution.x as _, self.resolution.y as _);

            let world_to_view = Mat4::look_to_rh(Vec3::ZERO, Vec3::Z, Vec3::Y);
            let projection = Mat4::orthographic_rh(-10., 10., -10., 10., -10., 10.);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.voxelizer_program, "world_to_view")
                    .as_ref(),
                false,
                world_to_view.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.voxelizer_program, "projection")
                    .as_ref(),
                false,
                projection.as_ref(),
            );

            gl.bind_image_texture(0, self.voxel_texture, 0, false, 0, WRITE_ONLY, RGBA16F);

            gl.disable(CULL_FACE);
            gl.disable(DEPTH_TEST);
            gl.disable(BLEND);

            for obj in objects {
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.voxelizer_program, "model_to_world")
                        .as_ref(),
                    false,
                    obj.get_transformation().as_ref(),
                );
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.voxelizer_program, "albedo")
                        .as_ref(),
                    obj.albedo.as_ref(),
                );
                obj.model.draw(
                    gl,
                    self.voxelizer_program,
                    "position",
                    Some("normal"),
                    Some("tex_coord"),
                );
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.enable(CULL_FACE);
        }
    }

    pub fn visualize(
        &self,
        gl: &Context,
        renderer: &QuadRenderer,
        camera: &Camera,
        screen_resolution: Vec2,
    ) {
        let aspect_ratio = screen_resolution.x / screen_resolution.y;
        let fov = camera.fov;
        let h = (fov * 0.5).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = viewport_height * aspect_ratio;

        let viewport_u = viewport_width * camera.right();
        let viewport_v = viewport_height * camera.up();
        let pixel_delta_u = viewport_u * (1.0 / screen_resolution.x);
        let pixel_delta_v = viewport_v * (1.0 / screen_resolution.y);

        let viewport_down_left =
            camera.position - camera.forward() - 0.5 * (viewport_u + viewport_v);
        let pixel_down_left = viewport_down_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        let projection = Mat4::orthographic_rh(-10., 10., -10., 10., -10., 10.);

        unsafe {
            //gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.msaa_fbo));
            //gl.read_buffer(COLOR_ATTACHMENT0);
            //gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
            //gl.clear(COLOR_BUFFER_BIT);
            //gl.blit_framebuffer(
            //    0,
            //    0,
            //    screen_resolution.x as _,
            //    screen_resolution.y as _,
            //    0,
            //    0,
            //    screen_resolution.x as _,
            //    screen_resolution.y as _,
            //    COLOR_BUFFER_BIT,
            //    LINEAR,
            //);
            //return;

            gl.use_program(Some(self.visualizing_program));
            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.viewport(0, 0, screen_resolution.x as _, screen_resolution.y as _);
            gl.clear(COLOR_BUFFER_BIT);

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.visualizing_program, "cam_pos")
                    .as_ref(),
                camera.position.as_ref(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.visualizing_program, "pixel_down_left")
                    .as_ref(),
                pixel_down_left.as_ref(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.visualizing_program, "pixel_delta_u")
                    .as_ref(),
                pixel_delta_u.as_ref(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.visualizing_program, "pixel_delta_v")
                    .as_ref(),
                pixel_delta_v.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.visualizing_program, "projection")
                    .as_ref(),
                false,
                projection.as_ref(),
            );

            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_3D, Some(self.voxel_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.visualizing_program, "voxel_tex")
                    .as_ref(),
                0,
            );

            renderer.draw_screen_quad(gl, self.visualizing_program);
        }
    }

    pub fn visualize_instanced(&self, gl: &Context, camera: &Camera, screen_resolution: Vec2) {
        unsafe {
            gl.use_program(Some(self.instanced_visualizing_program));
            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.viewport(0, 0, screen_resolution.x as _, screen_resolution.y as _);

            let aspect_ratio = screen_resolution.x / screen_resolution.y;
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.instanced_visualizing_program, "world_to_view")
                    .as_ref(),
                false,
                camera.view_transform().as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.instanced_visualizing_program, "projection")
                    .as_ref(),
                false,
                camera.perspective_transform(aspect_ratio).as_ref(),
            );
            gl.uniform_3_i32(
                gl.get_uniform_location(self.instanced_visualizing_program, "voxel_resolution")
                    .as_ref(),
                self.resolution.x as _,
                self.resolution.y as _,
                self.resolution.z as _,
            );

            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_3D, Some(self.voxel_texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.instanced_visualizing_program, "voxel_tex")
                    .as_ref(),
                0,
            );

            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.enable(CULL_FACE);
            gl.enable(DEPTH_TEST);
            self.cube_renderer.draw_instanced(
                gl,
                self.instanced_visualizing_program,
                (self.resolution.x * self.resolution.y * self.resolution.z) as _,
            );
            gl.disable(DEPTH_TEST);
        }
    }
}

struct CubeRenderer {
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    ebo: NativeBuffer,
}

impl CubeRenderer {
    fn new(gl: &Context) -> Self {
        let d = 0.5;
        let vertices = [
            Vec3::new(-d, -d, -d),
            Vec3::new(d, -d, -d),
            Vec3::new(-d, d, -d),
            Vec3::new(d, d, -d),
            Vec3::new(-d, -d, d),
            Vec3::new(d, -d, d),
            Vec3::new(-d, d, d),
            Vec3::new(d, d, d),
        ];
        let indices = [
            0, 2, 1, 2, 3, 1, 4, 5, 7, 6, 4, 7, 1, 3, 5, 3, 7, 5, 4, 6, 0, 6, 2, 0, 7, 3, 6, 3, 2,
            6, 1, 5, 4, 0, 1, 4,
        ];

        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&vertices), STATIC_DRAW);

            let ebo = gl.create_buffer().unwrap();
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                STATIC_DRAW,
            );
            Self { vao, vbo, ebo }
        }
    }

    fn draw_instanced(&self, gl: &Context, program: NativeProgram, count: i32) {
        unsafe {
            gl.bind_vertex_array(Some(self.vao));

            gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            let pos_loc = gl.get_attrib_location(program, "position").unwrap();
            gl.vertex_attrib_pointer_f32(pos_loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(pos_loc);

            gl.draw_elements_instanced(TRIANGLES, 36, UNSIGNED_INT, 0, count);
        }
    }
}
