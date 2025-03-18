use core::slice;
use std::ffi::CStr;

use glow::{
    Context, HasContext as _, COMPUTE_SHADER, MAP_READ_BIT, SHADER_STORAGE_BUFFER, STATIC_DRAW,
};
use microglut::{print_error, MicroGLUT};
use sdl2::video::Window;

struct HelloComputeShader {}

impl MicroGLUT for HelloComputeShader {
    fn init(gl: &Context, _window: &Window) -> Self {
        unsafe {
            let shader = gl.create_shader(COMPUTE_SHADER).unwrap();
            gl.shader_source(shader, include_str!("hello.cs"));
            gl.compile_shader(shader);
            let program = gl.create_program().unwrap();
            gl.attach_shader(program, shader);
            gl.link_program(program);
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
            print_error(gl, "create program").unwrap();

            gl.use_program(Some(program));

            let a = b"Hello \0\0\0\0\0\0".map(|c| c as i32);
            let b = [15, 10, 6, 0, -11, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

            let ssbo = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(ssbo));
            gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, bytemuck::cast_slice(&a), STATIC_DRAW);
            // ingemar: The "5" matches a "layout" number in the shader.
            // ingemar: (Can we ask the shader about the number? I must try that.)
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 5, Some(ssbo));

            // ingemar: Same for the other buffer, offsets, ID 6
            let ssbo2 = gl.create_buffer().unwrap();
            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(ssbo2));
            gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, bytemuck::cast_slice(&b), STATIC_DRAW);
            gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 6, Some(ssbo2));
            print_error(gl, "upload compute data").unwrap();

            gl.dispatch_compute(1, 1, 1);
            print_error(gl, "dispatch compute").unwrap();

            gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(ssbo));
            let ptr = gl.map_buffer_range(
                SHADER_STORAGE_BUFFER,
                0,
                12 * size_of::<i32>() as i32,
                MAP_READ_BIT,
            ) as *const _;
            print_error(gl, "get compute result").unwrap();
            let c: Vec<_> = slice::from_raw_parts(ptr as *const i32, 12)
                .iter()
                .map(|n| *n as u8)
                .collect();
            println!("{:?}", CStr::from_bytes_until_nul(&c).unwrap());
            sdl2::sys::quick_exit(0);
        }
    }

    fn display(&mut self, _gl: &Context, _window: &Window) {
        unimplemented!()
    }
}

fn main() {
    HelloComputeShader::sdl2_window("them compute shaders")
        .window_size(800, 800)
        .gl_version(4, 6)
        .start();
}
