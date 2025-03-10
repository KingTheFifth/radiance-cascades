use core::num;

use microglut::glow::{
    Context, HasContext, NativeFramebuffer, NativeTexture, COLOR_ATTACHMENT0, COLOR_ATTACHMENT1,
    COLOR_ATTACHMENT2, COLOR_ATTACHMENT3, COLOR_ATTACHMENT4, DEPTH_ATTACHMENT, DEPTH_COMPONENT,
    DEPTH_COMPONENT32, FLOAT, FRAMEBUFFER, LINEAR, NEAREST, NEAREST_MIPMAP_NEAREST, RENDERBUFFER,
    REPEAT, RG, RG16F, RG32F, RGB, RGB16F, RGBA, RGBA16F, RGBA32F, TEXTURE0, TEXTURE_2D,
    TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_S, TEXTURE_WRAP_T, UNSIGNED_BYTE,
};

pub struct SceneFBO {
    width: i32,
    height: i32,
    pub fb: NativeFramebuffer,
    pub albedo: NativeTexture,
    pub emissive: NativeTexture,
    pub normal: NativeTexture,
    pub depth_texture: NativeTexture,
    pub hi_z_texture: NativeTexture,
}

impl SceneFBO {
    pub fn init(gl: &Context, width: i32, height: i32) -> Self {
        unsafe {
            let fb = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(fb));

            let albedo = Self::create_texture(
                gl,
                width,
                height,
                RGBA32F as _,
                RGBA,
                COLOR_ATTACHMENT0,
                false,
            );
            let emissive = Self::create_texture(
                gl,
                width,
                height,
                RGBA32F as _,
                RGBA,
                COLOR_ATTACHMENT1,
                false,
            );
            let normal =
                Self::create_texture(gl, width, height, RG16F as _, RG, COLOR_ATTACHMENT2, false);

            let depth_texture = Self::create_texture(
                gl,
                width,
                height,
                DEPTH_COMPONENT32 as _,
                DEPTH_COMPONENT,
                DEPTH_ATTACHMENT,
                false,
            );

            let hi_z_texture =
                Self::create_texture(gl, width, height, RG32F as _, RG, COLOR_ATTACHMENT3, true);

            let draw_buffers = [
                COLOR_ATTACHMENT0,
                COLOR_ATTACHMENT1,
                COLOR_ATTACHMENT2,
                COLOR_ATTACHMENT3,
            ];
            gl.draw_buffers(&draw_buffers);

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.bind_renderbuffer(RENDERBUFFER, None);
            gl.bind_texture(TEXTURE_2D, None);

            SceneFBO {
                width,
                height,
                fb,
                albedo,
                emissive,
                normal,
                depth_texture,
                hi_z_texture,
            }
        }
    }

    pub fn bind_as_textures(&self, gl: &Context, first_texunit: u32) {
        unsafe {
            gl.active_texture(first_texunit);
            gl.bind_texture(TEXTURE_2D, Some(self.albedo));

            gl.active_texture(first_texunit + 1);
            gl.bind_texture(TEXTURE_2D, Some(self.emissive));

            gl.active_texture(first_texunit + 2);
            gl.bind_texture(TEXTURE_2D, Some(self.normal));

            gl.active_texture(first_texunit + 3);
            gl.bind_texture(TEXTURE_2D, Some(self.depth_texture));

            gl.active_texture(first_texunit + 4);
            gl.bind_texture(TEXTURE_2D, Some(self.hi_z_texture));
        }
    }

    fn create_texture(
        gl: &Context,
        width: i32,
        height: i32,
        internal_format: i32,
        format: u32, // Not actually used but must none the less be specified for correctness
        attachment: u32,
        mip_mapped: bool,
    ) -> NativeTexture {
        unsafe {
            let tex = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(tex));
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
            if mip_mapped {
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST_MIPMAP_NEAREST as _);
            } else {
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
            }
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                internal_format,
                width,
                height,
                0,
                format,
                UNSIGNED_BYTE,
                None,
            );
            if mip_mapped {
                gl.generate_mipmap(TEXTURE_2D);
            }
            gl.framebuffer_texture_2d(FRAMEBUFFER, attachment, TEXTURE_2D, Some(tex), 0);
            tex
        }
    }
}
