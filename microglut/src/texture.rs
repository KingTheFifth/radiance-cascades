use glow::{
    Context, HasContext as _, NativeTexture, LINEAR, LINEAR_MIPMAP_LINEAR, RGBA, TEXTURE_2D,
    TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, UNSIGNED_BYTE,
};

use crate::print_error;

#[derive(Debug)]
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
            // TODO:
            // let format = match image.depth {
            //     8 => RED,
            //     24 => RGB,
            //     32 => RGBA,
            //     d => panic!("unsupported bit depth: {}", d),
            // };
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as _,
                image.width as _,
                image.height as _,
                0,
                RGBA as _,
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

    pub fn id(&self) -> NativeTexture {
        self.id
    }
}
