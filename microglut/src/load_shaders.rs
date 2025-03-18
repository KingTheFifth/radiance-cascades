use std::path::Path;

use glow::{
    Context, HasContext as _, NativeProgram, COMPUTE_SHADER, FRAGMENT_SHADER, GEOMETRY_SHADER,
    TESS_CONTROL_SHADER, TESS_EVALUATION_SHADER, VERTEX_SHADER,
};

pub struct LoadShaders {
    vertex: String,
    fragment: String,
    geometry: Option<String>,
    tesselation_evaluation: Option<String>,
    tesselation_control: Option<String>,
}

impl LoadShaders {
    pub fn new(vertex: impl Into<String>, fragment: impl Into<String>) -> Self {
        LoadShaders {
            vertex: vertex.into(),
            fragment: fragment.into(),
            geometry: None,
            tesselation_evaluation: None,
            tesselation_control: None,
        }
    }

    pub fn new_from_path(
        vertex_shader_path: impl AsRef<Path>,
        fragment_shader_path: impl AsRef<Path>,
    ) -> Self {
        let vertex = std::fs::read_to_string(vertex_shader_path).unwrap();
        let fragment = std::fs::read_to_string(fragment_shader_path).unwrap();
        Self::new(vertex, fragment)
    }

    pub fn geometry(mut self, geometry_shader: impl Into<String>) -> Self {
        self.geometry = Some(geometry_shader.into());
        self
    }

    pub fn geometry_from_path(self, geometry_shader_path: impl AsRef<Path>) -> Self {
        let geometry = std::fs::read_to_string(geometry_shader_path).unwrap();
        self.geometry(geometry)
    }

    pub fn tesselation(
        mut self,
        tesselation_control_shader: impl Into<String>,
        tesselation_evaluation_shader: impl Into<String>,
    ) -> Self {
        self.tesselation_control = Some(tesselation_control_shader.into());
        self.tesselation_evaluation = Some(tesselation_evaluation_shader.into());
        self
    }

    pub fn compile(self, gl: &Context) -> NativeProgram {
        let shaders: Vec<_> = [
            (VERTEX_SHADER, self.vertex),
            (FRAGMENT_SHADER, self.fragment),
        ]
        .into_iter()
        .chain(self.geometry.map(|geometry| (GEOMETRY_SHADER, geometry)))
        .chain(
            self.tesselation_control
                .map(|tess_control| (TESS_CONTROL_SHADER, tess_control)),
        )
        .chain(
            self.tesselation_evaluation
                .map(|tess_evaluation| (TESS_EVALUATION_SHADER, tess_evaluation)),
        )
        .collect();

        compile_shaders(gl, &shaders)
    }
}

fn compile_shaders(gl: &Context, shaders: &[(u32, String)]) -> NativeProgram {
    unsafe {
        // TODO: should program be created outside?
        let program = gl.create_program().expect("Cannot create program");

        let shaders: Vec<_> = shaders
            .iter()
            .map(|(shader_type, shader_source)| {
                let shader = gl.create_shader(*shader_type).unwrap();
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shader
            })
            .collect();

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        program
    }
}

/// Simplified [LoadShaders] for just loading vertex and fragment shader.
pub fn load_shaders(
    gl: &Context,
    vertex_shader_source: impl Into<String>,
    fragment_shader_source: impl Into<String>,
) -> NativeProgram {
    LoadShaders::new(vertex_shader_source, fragment_shader_source).compile(gl)
}

pub fn load_compute_shader(gl: &Context, compute_shader: impl Into<String>) -> NativeProgram {
    compile_shaders(gl, &[(COMPUTE_SHADER, compute_shader.into())])
}
