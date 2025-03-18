use glam::{Vec2, Vec3};
use glow::{
    Context, HasContext as _, NativeProgram, COLOR_BUFFER_BIT, CULL_FACE, DEPTH_BUFFER_BIT,
    DEPTH_TEST,
};
use microglut::{elapsed_time, load_shaders, MicroGLUT, Model};
use sdl2::video::Window;

struct Ball {
    center: Vec2,
    radius: f32,
    speed: f32,
}

struct Metaballs {
    program: NativeProgram,
    ball_data: Vec<Ball>,
    quad: Model,
}

impl MicroGLUT for Metaballs {
    fn init(gl: &Context, _window: &Window) -> Self {
        let ball_data = (0..99)
            .map(|_| Ball {
                center: Vec2::new(
                    rand::random::<f32>() * 0.5 + 0.25,
                    rand::random::<f32>() * 0.5 + 0.25,
                ),
                radius: rand::random::<f32>() * 0.3 + 0.2,
                speed: rand::random::<f32>() * 0.5,
            })
            .collect();
        let ball_size = (0..99)
            .map(|_| rand::random::<f32>() * 0.05 + 0.03)
            .collect::<Vec<_>>();
        let quad_vertices = [
            Vec3::new(-1.0, 1.0, 0.99),
            Vec3::new(-1.0, -1.0, 0.99),
            Vec3::new(1.0, -1.0, 0.99),
            Vec3::new(1.0, 1.0, 0.99),
        ];
        let quad_indices = [0, 1, 2, 0, 2, 3];

        unsafe {
            gl.clear_color(0.2, 0.2, 0.5, 0.0);
            gl.disable(DEPTH_TEST);
            gl.disable(CULL_FACE);

            let program = load_shaders(
                gl,
                include_str!("metaballs99.vert"),
                include_str!("metaballs99.frag"),
            );
            gl.use_program(Some(program));

            let quad = Model::load_raw_data(
                gl,
                bytemuck::cast_slice(&quad_vertices),
                None,
                None,
                None,
                bytemuck::cast_slice(&quad_indices),
            );

            gl.uniform_1_f32_slice(
                gl.get_uniform_location(program, "ballsize").as_ref(),
                &ball_size,
            );

            Metaballs {
                program,
                ball_data,
                quad,
            }
        }
    }

    fn display(&mut self, gl: &Context, _window: &Window) {
        let t = elapsed_time();
        let ball_position = self
            .ball_data
            .iter()
            .map(|ball| {
                Vec2::new(
                    ball.center.x + (t * ball.speed).cos() * ball.radius,
                    ball.center.y + (t * ball.speed).sin() * ball.radius,
                )
            })
            .collect::<Vec<_>>();
        unsafe {
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            gl.uniform_2_f32_slice(
                gl.get_uniform_location(self.program, "balls").as_ref(),
                bytemuck::cast_slice(&ball_position),
            );
            self.quad.draw(gl, self.program, "inPosition", None, None);
        }
    }
}

fn main() {
    Metaballs::sdl2_window("Spicey metaballs").start();
}
