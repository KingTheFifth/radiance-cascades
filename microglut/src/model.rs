#![expect(clippy::missing_safety_doc)]
use std::{collections::HashMap, io::BufReader, path::Path};

use glam::{Vec2, Vec3};
use glow::{
    Buffer, Context, HasContext as _, NativeProgram, VertexArray, ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER, FLOAT, STATIC_DRAW, TEXTURE0, TEXTURE_2D, TRIANGLES, UNSIGNED_INT,
};

use crate::Texture;

type MaterialLoader = dyn Fn(&Path) -> tobj::MTLLoadResult;
type TextureLoader = dyn Fn(&str) -> Vec<u8>;

#[derive(Debug, Clone)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    material: Vec<Material>,
}

#[derive(Debug, Clone, Copy)]
pub struct Mesh {
    vertex_array: VertexArray,
    vertex_buffer: Buffer,
    normal_buffer: Option<Buffer>,
    texture_coordinate_buffer: Option<Buffer>,
    index_buffer: Buffer,
    num_indices: u32,
    material: Option<usize>, // index into Model.material, if any
}

impl Mesh {
    fn new(gl: &Context, mesh_data: tobj::Mesh, material: Option<usize>) -> Self {
        let (vertex_array, vertex_buffer, index_buffer) = unsafe {
            (
                gl.create_vertex_array().unwrap(),
                gl.create_buffer().unwrap(),
                gl.create_buffer().unwrap(),
            )
        };

        let normals = if mesh_data.normals.is_empty() {
            None
        } else {
            Some(mesh_data.normals)
        };
        let texture_coordinates = if mesh_data.texcoords.is_empty() {
            None
        } else {
            Some(mesh_data.texcoords)
        };
        let has_normals = normals.is_some();
        let has_texture_coordinates = texture_coordinates.is_some();

        let mesh = Mesh {
            vertex_array,
            vertex_buffer,
            index_buffer,
            num_indices: mesh_data.indices.len() as u32,
            normal_buffer: if has_normals {
                unsafe { Some(gl.create_buffer().unwrap()) }
            } else {
                None
            },
            texture_coordinate_buffer: if has_texture_coordinates {
                unsafe { Some(gl.create_buffer().unwrap()) }
            } else {
                None
            },
            material,
        };

        unsafe {
            mesh.vertex_data_f32(gl, &mesh_data.positions);
            mesh.index_data(gl, &mesh_data.indices);
            if let Some(normals) = &normals {
                mesh.normal_data_f32(gl, normals);
            }
            if let Some(texture_coordinates) = &texture_coordinates {
                mesh.texture_data_f32(gl, texture_coordinates);
            }
        }

        mesh
    }

    pub fn num_indices(&self) -> usize {
        self.num_indices as usize
    }

    pub unsafe fn vertex_data(&self, gl: &Context, data: &[Vec3]) {
        self.vertex_data_f32(gl, bytemuck::cast_slice(data))
    }

    pub unsafe fn vertex_data_f32(&self, gl: &Context, data: &[f32]) {
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
        gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(data), STATIC_DRAW);
    }

    pub unsafe fn index_data(&self, gl: &Context, data: &[u32]) {
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        gl.buffer_data_u8_slice(
            ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(data),
            STATIC_DRAW,
        );
    }

    pub unsafe fn normal_data(&self, gl: &Context, data: &[Vec3]) {
        self.normal_data_f32(gl, bytemuck::cast_slice(data))
    }

    pub unsafe fn normal_data_f32(&self, gl: &Context, data: &[f32]) {
        // TODO: handle no buffer
        if let Some(normal_buffer) = self.normal_buffer {
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(normal_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(data), STATIC_DRAW);
        }
    }

    pub unsafe fn texture_data(&self, gl: &Context, data: &[Vec2]) {
        self.texture_data_f32(gl, bytemuck::cast_slice(data))
    }

    pub unsafe fn texture_data_f32(&self, gl: &Context, data: &[f32]) {
        // TODO: handle no buffer
        if let Some(texture_coordinate_buffer) = self.texture_coordinate_buffer {
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(texture_coordinate_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(data), STATIC_DRAW);
        }
    }

    pub fn bind(
        &self,
        gl: &Context,
        program: NativeProgram,
        vertex_binding: &str,
        normal_binding: Option<&str>,
        texture_binding: Option<&str>,
    ) {
        unsafe {
            gl.bind_vertex_array(Some(self.vertex_array));

            gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            let loc = gl.get_attrib_location(program, vertex_binding).unwrap();
            gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(loc);

            if let Some(normal_binding) = normal_binding {
                if let Some(loc) = gl.get_attrib_location(program, normal_binding) {
                    gl.bind_buffer(ARRAY_BUFFER, Some(self.normal_buffer.unwrap()));
                    gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(loc);
                } else {
                    // TODO: warn once
                }
            }

            if let Some(texture_binding) = texture_binding {
                if let Some(loc) = gl.get_attrib_location(program, texture_binding) {
                    gl.bind_buffer(ARRAY_BUFFER, Some(self.texture_coordinate_buffer.unwrap()));
                    gl.vertex_attrib_pointer_f32(loc, 2, FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(loc);
                } else {
                    // TODO: warn once
                }
            }
        }
    }

    pub fn draw(
        &self,
        gl: &Context,
        program: NativeProgram,
        vertex_binding: &str,
        normal_binding: Option<&str>,
        texture_binding: Option<&str>,
    ) {
        self.bind(gl, program, vertex_binding, normal_binding, texture_binding);
        unsafe { gl.draw_elements(TRIANGLES, self.num_indices as _, UNSIGNED_INT, 0) }
    }
}

#[derive(Debug, Clone, Copy)]
struct Material {
    ambient: Option<Vec3>,
    diffuse: Option<Vec3>,
    specular: Option<Vec3>,
    shininess: Option<f32>,
    dissolve: Option<f32>,
    optical_density: Option<f32>,
    ambient_texture: Option<Texture>,
    diffuse_texture: Option<Texture>,
    specular_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    shininess_texture: Option<Texture>,
    dissolve_texture: Option<Texture>,
    illumination_model: Option<u8>,
}

pub struct MaterialBindings {
    pub ambient: Option<String>,
    pub diffuse: Option<String>,
    pub specular: Option<String>,
    pub shininess: Option<String>,
    pub dissolve: Option<String>,
    pub optical_density: Option<String>,
    pub ambient_texture: Option<(String, u32)>,
    pub diffuse_texture: Option<(String, u32)>,
    pub specular_texture: Option<(String, u32)>,
    pub normal_texture: Option<(String, u32)>,
    pub shininess_texture: Option<(String, u32)>,
    pub dissolve_texture: Option<(String, u32)>,
    pub illumination_model: Option<String>,
}

impl Material {
    fn new(gl: &Context, material: tobj::Material, texture_loader: Option<&TextureLoader>) -> Self {
        // TODO: is the "main" texture always the name?
        if let Some(tex_loader) = texture_loader {
            let ambient_texture = material
                .ambient_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            let diffuse_texture = material
                .diffuse_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            let specular_texture = material
                .specular_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            let normal_texture = material
                .normal_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            let shininess_texture = material
                .shininess_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            let dissolve_texture = material
                .dissolve_texture
                .map(|texture_name| tex_loader(&texture_name))
                .map(|data| Texture::load(gl, &data, true));
            Material {
                ambient: material.ambient.map(Vec3::from_array),
                diffuse: material.diffuse.map(Vec3::from_array),
                specular: material.specular.map(Vec3::from_array),
                shininess: material.shininess,
                dissolve: material.dissolve,
                optical_density: material.optical_density,
                ambient_texture,
                diffuse_texture,
                specular_texture,
                normal_texture,
                shininess_texture,
                dissolve_texture,
                illumination_model: material.illumination_model,
            }
        } else {
            Material {
                ambient: material.ambient.map(Vec3::from_array),
                diffuse: material.diffuse.map(Vec3::from_array),
                specular: material.specular.map(Vec3::from_array),
                shininess: material.shininess,
                dissolve: material.dissolve,
                optical_density: material.optical_density,
                ambient_texture: None,
                diffuse_texture: None,
                specular_texture: None,
                normal_texture: None,
                shininess_texture: None,
                dissolve_texture: None,
                illumination_model: material.illumination_model,
            }
        }
    }

    fn bind(&self, gl: &Context, program: NativeProgram, bindings: &MaterialBindings) {
        unsafe {
            if let Some(ambient_binding) = &bindings.ambient {
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(program, &ambient_binding).as_ref(),
                    self.ambient.unwrap_or_default().as_ref(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", ambient_binding))
                        .as_ref(),
                    self.ambient.is_some() as i32,
                );
            }
            if let Some(diffuse_binding) = &bindings.diffuse {
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(program, &diffuse_binding).as_ref(),
                    self.diffuse.unwrap_or_default().as_ref(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", diffuse_binding))
                        .as_ref(),
                    self.diffuse.is_some() as i32,
                );
            }
            if let Some(specular_binding) = &bindings.specular {
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(program, &specular_binding).as_ref(),
                    self.specular.unwrap_or_default().as_ref(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", specular_binding))
                        .as_ref(),
                    self.specular.is_some() as i32,
                );
            }
            if let Some(shininess_binding) = &bindings.shininess {
                gl.uniform_1_f32(
                    gl.get_uniform_location(program, &shininess_binding)
                        .as_ref(),
                    self.shininess.unwrap_or_default(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", shininess_binding))
                        .as_ref(),
                    self.shininess.is_some() as i32,
                );
            }
            if let Some(dissolve_binding) = &bindings.dissolve {
                gl.uniform_1_f32(
                    gl.get_uniform_location(program, &dissolve_binding).as_ref(),
                    self.dissolve.unwrap_or_default(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", dissolve_binding))
                        .as_ref(),
                    self.dissolve.is_some() as i32,
                );
            }
            if let Some(optical_density_binding) = &bindings.optical_density {
                gl.uniform_1_f32(
                    gl.get_uniform_location(program, &optical_density_binding)
                        .as_ref(),
                    self.optical_density.unwrap_or_default(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", optical_density_binding))
                        .as_ref(),
                    self.optical_density.is_some() as i32,
                );
            }

            if let Some((ambient_texture_binding, texture_unit)) = &bindings.ambient_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.ambient_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &ambient_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", ambient_texture_binding))
                        .as_ref(),
                    self.ambient_texture.is_some() as i32,
                );
            }
            if let Some((diffuse_texture_binding, texture_unit)) = &bindings.diffuse_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.diffuse_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &diffuse_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", diffuse_texture_binding))
                        .as_ref(),
                    self.diffuse_texture.is_some() as i32,
                );
            }
            if let Some((specular_texture_binding, texture_unit)) = &bindings.specular_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.specular_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &specular_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", specular_texture_binding))
                        .as_ref(),
                    self.specular_texture.is_some() as i32,
                );
            }
            if let Some((normal_texture_binding, texture_unit)) = &bindings.normal_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.normal_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &normal_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", normal_texture_binding))
                        .as_ref(),
                    self.normal_texture.is_some() as i32,
                );
            }
            if let Some((shininess_texture_binding, texture_unit)) = &bindings.shininess_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.shininess_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &shininess_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", shininess_texture_binding))
                        .as_ref(),
                    self.shininess_texture.is_some() as i32,
                );
            }
            if let Some((dissolve_texture_binding, texture_unit)) = &bindings.dissolve_texture {
                gl.active_texture(TEXTURE0 + texture_unit);
                gl.bind_texture(TEXTURE_2D, self.dissolve_texture.map(|t| t.id()));
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &dissolve_texture_binding)
                        .as_ref(),
                    *texture_unit as i32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", dissolve_texture_binding))
                        .as_ref(),
                    self.dissolve_texture.is_some() as i32,
                );
            }

            if let Some(illumination_model_binding) = &bindings.illumination_model {
                gl.uniform_1_u32(
                    gl.get_uniform_location(program, &illumination_model_binding)
                        .as_ref(),
                    self.illumination_model.unwrap_or_default() as u32,
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(
                        program,
                        &format!("has_{}", illumination_model_binding),
                    )
                    .as_ref(),
                    self.illumination_model.is_some() as i32,
                );
            }
        }
    }
}

impl Model {
    pub fn load_raw_data(
        gl: &Context,
        vertices: &[f32],
        normals: Option<&[f32]>,
        texture_coordinates: Option<&[f32]>,
        _colors: Option<&[f32]>,
        indices: &[u32],
    ) -> Self {
        let has_normals = normals.is_some();
        let has_texture_coordinates = texture_coordinates.is_some();

        let (vertex_array, vertex_buffer, index_buffer) = unsafe {
            (
                gl.create_vertex_array().unwrap(),
                gl.create_buffer().unwrap(),
                gl.create_buffer().unwrap(),
            )
        };

        let mesh = Mesh {
            vertex_array,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            normal_buffer: if has_normals {
                unsafe { Some(gl.create_buffer().unwrap()) }
            } else {
                None
            },
            texture_coordinate_buffer: if has_texture_coordinates {
                unsafe { Some(gl.create_buffer().unwrap()) }
            } else {
                None
            },
            material: None,
        };

        unsafe {
            mesh.vertex_data_f32(gl, vertices);
            mesh.index_data(gl, indices);
            if let Some(normals) = &normals {
                mesh.normal_data_f32(gl, normals);
            }
            if let Some(texture_coordinates) = &texture_coordinates {
                mesh.texture_data_f32(gl, texture_coordinates);
            }
        }

        Model {
            meshes: vec![mesh],
            material: Vec::new(),
        }
    }

    // TODO: load_obj builder function? if more params are needed
    pub fn load_obj_data(
        gl: &Context,
        data: &[u8],
        material_loader: Option<&MaterialLoader>,
        texture_loader: Option<&TextureLoader>,
    ) -> Self {
        let model = tobj::load_obj_buf(
            &mut BufReader::new(data),
            &tobj::LoadOptions {
                // use the same index for every vertex/normal/texture coordinate
                single_index: true,
                ..tobj::GPU_LOAD_OPTIONS
            },
            |path| match &material_loader {
                Some(f) => f(path),
                None => Ok((Vec::new(), HashMap::new())),
            },
        )
        .unwrap();

        let meshes = model
            .0
            .into_iter()
            .map(|mesh| {
                let material_id = mesh.mesh.material_id;
                Mesh::new(gl, mesh.mesh, material_id)
            })
            .collect();
        // TODO: can we have materials without textures?
        let material = model
            .1
            .unwrap()
            .into_iter()
            .map(|material| Material::new(gl, material, texture_loader))
            .collect();

        Model { meshes, material }
    }

    pub fn draw(
        &self,
        gl: &Context,
        program: NativeProgram,
        vertex_binding: &str,
        normal_binding: Option<&str>,
        texture_binding: Option<&str>,
        material_bindings: Option<&MaterialBindings>,
    ) {
        for mesh in &self.meshes {
            if let Some(material) = mesh.material {
                if let Some(m) = material_bindings {
                    self.material[material].bind(gl, program, m);
                }
            }
            mesh.draw(gl, program, vertex_binding, normal_binding, texture_binding);
        }
    }

    pub fn draw_mesh(
        &self,
        gl: &Context,
        mesh_idx: usize,
        program: NativeProgram,
        vertex_binding: &str,
        normal_binding: Option<&str>,
        texture_binding: Option<&str>,
        material_bindings: Option<&MaterialBindings>,
    ) {
        let Some(mesh) = self.meshes.get(mesh_idx) else {
            return;
        };
        if let Some(material) = mesh.material {
            if let Some(m) = material_bindings {
                self.material[material].bind(gl, program, m);
            }
        }
        mesh.draw(gl, program, vertex_binding, normal_binding, texture_binding);
    }
}
