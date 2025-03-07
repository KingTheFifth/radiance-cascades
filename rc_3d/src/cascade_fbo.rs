use microglut::{
    glam::Vec2,
    glow::{
        Context, HasContext, NativeFramebuffer, NativeTexture, CLAMP_TO_EDGE, COLOR_ATTACHMENT0,
        DEPTH_ATTACHMENT, FRAMEBUFFER, LINEAR, NEAREST, RENDERBUFFER, RGBA, RGBA16F, TEXTURE_2D,
        TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_S, TEXTURE_WRAP_T, UNSIGNED_BYTE,
    },
};

pub struct CascadeFBO {
    pub fb: NativeFramebuffer,
    pub cascades: Vec<NativeTexture>,
}

impl CascadeFBO {
    pub fn new(gl: &Context, c0_res: Vec2, num_cascades: i32) -> Self {
        unsafe {
            let fb = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(fb));

            //let rb = gl.create_renderbuffer().unwrap();
            //gl.bind_renderbuffer(RENDERBUFFER, Some(rb));
            //gl.framebuffer_renderbuffer(FRAMEBUFFER, DEPTH_ATTACHMENT, RENDERBUFFER, Some(rb));
            //gl.bind_renderbuffer(RENDERBUFFER, None);

            let cascades = (0..num_cascades)
                .into_iter()
                .map(|i| {
                    let tex = gl.create_texture().unwrap();
                    gl.bind_texture(TEXTURE_2D, Some(tex));
                    gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as _);
                    gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as _);
                    gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
                    gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
                    gl.tex_image_2d(
                        TEXTURE_2D,
                        0,
                        RGBA16F as _,
                        c0_res.x as _,
                        (c0_res.y / 2.0_f32.powi(i)) as _,
                        0,
                        RGBA,
                        UNSIGNED_BYTE,
                        None,
                    );
                    //gl.tex_storage_2d(
                    //    TEXTURE_2D,
                    //    0,
                    //    RGBA16F,
                    //    c0_res.x as _,
                    //    (c0_res.y / 2.0_f32.powi(i)) as _,
                    //);
                    tex
                })
                .collect();

            gl.draw_buffers(&[COLOR_ATTACHMENT0]);
            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.bind_texture(TEXTURE_2D, None);
            CascadeFBO { fb, cascades }
        }
    }

    pub fn bind_cascade_as_texture(&self, gl: &Context, cascade: usize, texture_unit: u32) {
        unsafe {
            gl.active_texture(texture_unit);
            gl.bind_texture(TEXTURE_2D, Some(self.cascades[cascade]));
        }
    }

    pub fn bind_cascade_as_output(&self, gl: &Context, cascade: usize) {
        unsafe {
            gl.bind_framebuffer(FRAMEBUFFER, Some(self.fb));
            gl.framebuffer_texture(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
                Some(self.cascades[cascade]),
                0,
            );
        };
    }
}
