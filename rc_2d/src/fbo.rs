use microglut::glow::{
    Context, HasContext, NativeFramebuffer, NativeRenderbuffer, NativeTexture, COLOR_ATTACHMENT0,
    COLOR_ATTACHMENT1, DEPTH_ATTACHMENT, DEPTH_COMPONENT24, FRAMEBUFFER, LINEAR, RENDERBUFFER,
    REPEAT, RGBA, RGBA32F, TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_S,
    TEXTURE_WRAP_T, UNSIGNED_BYTE,
};

pub struct SceneFBO {
    width: i32,
    height: i32,
    pub fb: NativeFramebuffer,
    pub textures: Vec<NativeTexture>,
    #[expect(unused)]
    rb: NativeRenderbuffer,
}

impl SceneFBO {
    pub fn init(gl: &Context, width: i32, height: i32, num_textures: u32) -> Self {
        unsafe {
            let fb = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(fb));

            let mut textures = vec![];
            for i in 0..num_textures {
                let tex = gl.create_texture().unwrap();
                textures.push(tex);
                gl.bind_texture(TEXTURE_2D, Some(tex));
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, REPEAT as _);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, REPEAT as _);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
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
                gl.framebuffer_texture_2d(
                    FRAMEBUFFER,
                    COLOR_ATTACHMENT0 + i,
                    TEXTURE_2D,
                    Some(tex),
                    0,
                );
            }
            let draw_buffers: Vec<u32> = (0..num_textures).map(|i| COLOR_ATTACHMENT0 + i).collect();
            gl.draw_buffers(&draw_buffers);

            let rb = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(RENDERBUFFER, Some(rb));
            gl.renderbuffer_storage(RENDERBUFFER, DEPTH_COMPONENT24, width, height);
            gl.framebuffer_renderbuffer(FRAMEBUFFER, DEPTH_ATTACHMENT, RENDERBUFFER, Some(rb));

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.bind_renderbuffer(RENDERBUFFER, None);
            gl.bind_texture(TEXTURE_2D, None);

            SceneFBO {
                width,
                height,
                fb,
                textures,
                rb,
            }
        }
    }

    pub fn bind_as_textures(&self, gl: &Context, first_texunit: u32) {
        unsafe {
            for i in 0..self.textures.len() {
                gl.active_texture(first_texunit + (i as u32));
                gl.bind_texture(TEXTURE_2D, Some(self.textures[i]));
            }
        }
    }
}
