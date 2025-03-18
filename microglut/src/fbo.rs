use glow::{
    Context, HasContext as _, NativeFramebuffer, NativeRenderbuffer, NativeTexture,
    COLOR_ATTACHMENT0, DEPTH_ATTACHMENT, DEPTH_COMPONENT24, FRAMEBUFFER, LINEAR, NEAREST,
    RENDERBUFFER, REPEAT, RGBA, RGBA32F, TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER,
    TEXTURE_WRAP_S, TEXTURE_WRAP_T, UNSIGNED_BYTE,
};

pub struct FBO {
    width: i32,
    height: i32,
    fb: NativeFramebuffer,
    tex: NativeTexture,
    #[expect(unused)]
    rb: NativeRenderbuffer,
}

impl FBO {
    pub fn init(gl: &Context, width: i32, height: i32, filter_nn: bool) -> Self {
        unsafe {
            let fb = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(fb));
            let tex = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(tex));
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT as _);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT as _);
            if filter_nn {
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as _);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as _);
            } else {
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
            }
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA32F as _,
                width,
                height,
                0,
                RGBA,
                UNSIGNED_BYTE,
                None,
            );
            gl.framebuffer_texture_2d(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, Some(tex), 0);

            // renderbuffer
            let rb = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(RENDERBUFFER, Some(rb));
            gl.renderbuffer_storage(RENDERBUFFER, DEPTH_COMPONENT24, width, height);
            gl.framebuffer_renderbuffer(FRAMEBUFFER, DEPTH_ATTACHMENT, RENDERBUFFER, Some(rb));
            // TODO: check framebuffer status

            gl.bind_framebuffer(FRAMEBUFFER, Some(fb));

            Self {
                width,
                height,
                fb,
                tex,
                rb,
            }
        }
    }
}

// TODO: either FBO or screen size?
pub fn bind_output_fbo(gl: &Context, output: Option<&FBO>, screen_width: i32, screen_height: i32) {
    unsafe {
        match output {
            None => {
                gl.bind_framebuffer(FRAMEBUFFER, None);
                gl.viewport(0, 0, screen_width, screen_height);
            }
            Some(fbo) => {
                gl.bind_framebuffer(FRAMEBUFFER, Some(fbo.fb));
                gl.viewport(0, 0, fbo.width, fbo.height);
            }
        }
    }
}

pub unsafe fn bind_texture_fbo(gl: &Context, fbo: &FBO, tex_unit: u32) {
    unsafe {
        gl.active_texture(tex_unit);
        gl.bind_texture(TEXTURE_2D, Some(fbo.tex));
    }
}
