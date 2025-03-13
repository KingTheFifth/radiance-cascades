use microglut::{
    glam::{Mat4, Vec2, Vec3},
    glow::{
        Context, HasContext, NativeFramebuffer, NativeProgram, NativeTexture, BLEND, CLAMP_TO_EDGE,
        COLOR_ATTACHMENT0, COLOR_BUFFER_BIT, CULL_FACE, DEPTH_ATTACHMENT, DEPTH_COMPONENT16,
        DEPTH_TEST, DRAW_FRAMEBUFFER, FRAMEBUFFER, LINEAR, READ_FRAMEBUFFER, RENDERBUFFER, RGBA,
        RGBA16F, RGBA8, TEXTURE_2D, TEXTURE_2D_MULTISAMPLE, TEXTURE_3D, TEXTURE_MAG_FILTER,
        TEXTURE_MIN_FILTER, TEXTURE_WRAP_R, TEXTURE_WRAP_S, TEXTURE_WRAP_T, UNSIGNED_BYTE,
        WRITE_ONLY,
    },
    LoadShaders,
};

use crate::object::Object;

pub struct Voxelizer {
    resolution: Vec3,
    voxel_texture: NativeTexture,
    program: NativeProgram,

    // An MSAA render target is needed for an approximation of conservative rasterization
    msaa_fbo: NativeFramebuffer,
}

impl Voxelizer {
    pub fn new(gl: &Context, resolution: Vec3) -> Self {
        unsafe {
            let program =
                LoadShaders::new(include_str!("voxelize.vert"), include_str!("voxelize.frag"))
                    .geometry(include_str!("voxelize.geom"))
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

            Self {
                resolution,
                voxel_texture,
                program,
                msaa_fbo,
            }
        }
    }

    pub fn voxelize(&self, gl: &Context, objects: &Vec<Object>) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.msaa_fbo));
            gl.viewport(0, 0, self.resolution.x as _, self.resolution.y as _);

            let world_to_view = Mat4::look_to_rh(Vec3::ZERO, Vec3::Z, Vec3::Y);
            let projection = Mat4::orthographic_rh(-10., 10., -10., 10., -10., 10.);

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "world_to_view")
                    .as_ref(),
                false,
                world_to_view.as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "projection").as_ref(),
                false,
                projection.as_ref(),
            );

            gl.bind_image_texture(0, self.voxel_texture, 0, false, 0, WRITE_ONLY, RGBA16F);

            gl.disable(CULL_FACE);
            gl.disable(DEPTH_TEST);
            gl.disable(BLEND);

            for obj in objects {
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program, "model_to_world")
                        .as_ref(),
                    false,
                    obj.get_transformation().as_ref(),
                );
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.program, "albedo").as_ref(),
                    obj.albedo.as_ref(),
                );
                obj.model.draw(
                    gl,
                    self.program,
                    "position",
                    Some("normal"),
                    Some("tex_coord"),
                );
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.enable(CULL_FACE);
        }
    }

    pub fn blit_to_screen(&self, gl: &Context, screen_resolution: Vec2) {
        unsafe {
            gl.bind_framebuffer(READ_FRAMEBUFFER, Some(self.msaa_fbo));
            gl.read_buffer(COLOR_ATTACHMENT0);
            gl.bind_framebuffer(DRAW_FRAMEBUFFER, None);
            gl.viewport(0, 0, screen_resolution.x as _, screen_resolution.y as _);
            gl.clear(COLOR_BUFFER_BIT);
            // The blitting dimensions must match when using multisampled FBO
            gl.blit_framebuffer(
                0,
                0,
                self.resolution.x as _,
                self.resolution.y as _,
                0,
                0,
                self.resolution.x as _,
                self.resolution.y as _,
                COLOR_BUFFER_BIT,
                LINEAR,
            );
        }
    }
}
