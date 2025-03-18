use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use glow::{
    HasContext, NativeProgram, BACK, COLOR_BUFFER_BIT, CULL_FACE, DEPTH_BUFFER_BIT, DEPTH_TEST,
};
use microglut::{print_error, util::arb_rotate, LoadShaders, MicroGLUT, Model};
use sdl2::mouse::MouseButton;

struct Demo {
    programs: [NativeProgram; 4],
    current_program: usize,
    m_model_to_world: Mat4,
    model: Model,
    mouse_down: bool,
}

impl MicroGLUT for Demo {
    fn init(gl: &glow::Context, _window: &sdl2::video::Window) -> Self {
        let programs = [
            LoadShaders::new(include_str!("minimal.vert"), include_str!("minimal.frag"))
                .geometry(include_str!("passthrough.gs"))
                .compile(gl),
            LoadShaders::new(include_str!("minimal.vert"), include_str!("minimal.frag"))
                .geometry(include_str!("flatshading.gs"))
                .compile(gl),
            LoadShaders::new(include_str!("minimal.vert"), include_str!("minimal.frag"))
                .geometry(include_str!("cracking.gs"))
                .compile(gl),
            LoadShaders::new(include_str!("minimal.vert"), include_str!("black.frag"))
                .geometry(include_str!("wireframe.gs"))
                .compile(gl),
        ];
        let model = Model::load_obj_data(gl, include_bytes!("bunnyplus.obj"), None, None);
        // TODO: CenterModel
        let proj_matrix = Mat4::perspective_rh(PI / 2.0, 1.0, 0.1, 300.0);

        unsafe {
            for program in &programs {
                gl.use_program(Some(*program));
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(*program, "projMatrix").as_ref(),
                    false,
                    proj_matrix.as_ref(),
                );
            }

            gl.clear_color(1.0, 1.0, 1.0, 0.0);
            gl.enable(DEPTH_TEST);
            gl.enable(CULL_FACE);
            gl.cull_face(BACK);
            print_error(gl, "init").unwrap();
        }

        Demo {
            programs,
            current_program: 1,
            m_model_to_world: Mat4::IDENTITY,
            model,
            mouse_down: false,
        }
    }

    fn display(&mut self, gl: &glow::Context, _window: &sdl2::video::Window) {
        let world_to_view = Mat4::look_at_rh(Vec3::new(0.0, 0.4, 1.4), Vec3::ZERO, Vec3::Y);
        let model_to_view = world_to_view * self.m_model_to_world;
        let program = self.programs[self.current_program];

        unsafe {
            gl.use_program(Some(program));
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(program, "modelViewMatrix").as_ref(),
                false,
                model_to_view.as_ref(),
            );
            self.model
                .draw(gl, program, "inPosition", Some("inNormal"), None);

            print_error(gl, "display").unwrap();
        }
    }

    #[cfg(feature = "imgui")]
    fn ui(&mut self, _gl: &glow::Context, ui: &mut imgui::Ui) {
        ui.window("debug").build(|| {
            for (label, button_value) in [
                ("Nothing", 0),
                ("Flat shading", 1),
                ("Cracking", 2),
                ("Wireframe", 3),
            ] {
                ui.radio_button(label, &mut self.current_program, button_value);
            }
        });
    }

    fn mouse_up(&mut self, _button: MouseButton, _x: i32, _y: i32) {
        self.mouse_down = false;
    }

    fn mouse_down(&mut self, _button: MouseButton, _: i32, _: i32) {
        self.mouse_down = true;
    }

    // TODO: do not interact if mouse clicked on ui?
    fn mouse_moved_rel(&mut self, xrel: i32, yrel: i32) {
        if self.mouse_down {
            let p = Vec3::new(yrel as f32, xrel as f32, 0.0);
            let m = arb_rotate(p, (p.x * p.x + p.y * p.y).sqrt() / 50.0);
            self.m_model_to_world = m * self.m_model_to_world;
        }
    }
}

fn main() {
    Demo::sdl2_window("GL3 geometry shading example")
        .window_size(800, 800)
        .start();
}
