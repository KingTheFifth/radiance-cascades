use glow::{
    Context, HasContext as _, NativeTexture, LINEAR, LINEAR_MIPMAP_LINEAR, RED, RG, RGB, RGBA,
    TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, UNSIGNED_BYTE,
};

use crate::print_error;

#[derive(Debug, Clone, Copy)]
pub struct Texture {
    id: NativeTexture,
}

impl Texture {
    pub fn load(gl: &Context, data: &[u8], gen_mipmap: bool) -> Self {
        use stb_image::image::{load_from_memory, LoadResult};
        let image = match load_from_memory(data) {
            LoadResult::Error(e) => panic!("{}", e),
            LoadResult::ImageU8(image) => image,
            LoadResult::ImageF32(_image) => todo!(),
        };

        unsafe {
            let tex_id = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(tex_id));
            print_error(gl, "texture bind_texture").unwrap();
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
            print_error(gl, "texture param min filter").unwrap();
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
            print_error(gl, "texture param mag filter").unwrap();

            let format = match image.depth {
                1 => RED,
                2 => RG,
                3 => RGB,
                4 => RGBA,
                d => panic!("unsupported bit depth: {}", d),
            };

            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as _,
                image.width as _,
                image.height as _,
                0,
                format as _,
                UNSIGNED_BYTE,
                Some(&image.data),
            );
            print_error(gl, "texture image_2d").unwrap();
            if gen_mipmap {
                gl.generate_mipmap(TEXTURE_2D);
                print_error(gl, "texture gen mipmap").unwrap();
                gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR_MIPMAP_LINEAR as _);
                print_error(gl, "texture mipmap min filter").unwrap();
            }
            Texture { id: tex_id }
        }
    }

    pub fn load_with_parameters(
        gl: &Context,
        data: &[u8],
        int_params: &[(u32, i32)],
        float_params: &[(u32, f32)],
        gen_mipmap: bool,
    ) -> Self {
        use stb_image::image::{load_from_memory, LoadResult};
        let image = match load_from_memory(data) {
            LoadResult::Error(e) => panic!("{}", e),
            LoadResult::ImageU8(image) => image,
            LoadResult::ImageF32(_image) => todo!(),
        };

        unsafe {
            let tex_id = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(tex_id));

            for (parameter, value) in int_params {
                gl.tex_parameter_i32(TEXTURE_2D, *parameter, *value);
                print_error(gl, &format!("texture param {}", parameter)).unwrap();
            }

            for (parameter, value) in float_params {
                gl.tex_parameter_f32(TEXTURE_2D, *parameter, *value);
                print_error(gl, &format!("texture param {}", parameter)).unwrap();
            }

            let format = match image.depth {
                1 => RED,
                2 => RG,
                3 => RGB,
                4 => RGBA,
                d => panic!("unsupported bit depth: {}", d),
            };

            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as _,
                image.width as _,
                image.height as _,
                0,
                format as _,
                UNSIGNED_BYTE,
                Some(&image.data),
            );
            print_error(gl, "texture image_2d").unwrap();
            if gen_mipmap {
                gl.generate_mipmap(TEXTURE_2D);
                print_error(gl, "texture gen mipmap").unwrap();
            }
            Texture { id: tex_id }
        }
    }

    pub fn id(&self) -> NativeTexture {
        self.id
    }
}
