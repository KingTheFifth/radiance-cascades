#![expect(clippy::missing_safety_doc)]
use core::fmt;
use std::{
    collections::HashMap,
    error::Error,
    io::{BufRead, BufReader},
    path::Path,
    str::{FromStr, SplitWhitespace},
};

use glam::{Vec2, Vec3};
use glow::{
    Buffer, Context, HasContext as _, NativeProgram, VertexArray, ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER, FLOAT, LINEAR, LINEAR_MIPMAP_LINEAR, MAX_TEXTURE_MAX_ANISOTROPY_EXT,
    REPEAT, STATIC_DRAW, TEXTURE0, TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MAX_ANISOTROPY_EXT,
    TEXTURE_MIN_FILTER, TEXTURE_WRAP_R, TEXTURE_WRAP_S, TRIANGLES, UNSIGNED_INT,
};

use crate::Texture;

type MaterialLoader = dyn Fn(&Path) -> tobj::MTLLoadResult;
type TextureLoader = dyn Fn(&str) -> Vec<u8>;
type TangentLoader = dyn Fn(&str) -> Vec<u8>;

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
    tangent_buffer: Option<Buffer>,
    bitangent_buffer: Option<Buffer>,
    texture_coordinate_buffer: Option<Buffer>,
    index_buffer: Buffer,
    num_indices: u32,
    pub material: Option<usize>, // index into Model.material, if any
}

fn is_invalid_tangent(v: Vec3) -> bool {
    return !v.is_finite() || (v.cmpgt(Vec3::ONE * -0.5).all() && v.cmplt(Vec3::ONE * 0.5).all());
}

fn is_invalid_bitangent(v: Vec3) -> bool {
    return is_invalid_tangent(v);
}

fn reconstruct_tangent(tangent: Vec3, bitangent: Vec3, normal: Vec3) -> Vec3 {
    if is_invalid_tangent(tangent) && !is_invalid_bitangent(bitangent) {
        normal.cross(bitangent).normalize()
    } else {
        tangent
    }
}

fn reconstruct_bitangent(tangent: Vec3, bitangent: Vec3, normal: Vec3) -> Vec3 {
    if is_invalid_bitangent(bitangent) && !is_invalid_tangent(tangent) {
        tangent.cross(normal).normalize()
    } else {
        bitangent
    }
}

fn parse_vec3(word: SplitWhitespace) -> Result<Vec3, LoadError> {
    let v = word
        .take(3)
        .map(FromStr::from_str)
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|_| LoadError::ParseError)
        .map(|v| Vec3::from_slice(&v))
        .unwrap();
    Ok(v)
}

#[derive(Debug)]
pub enum LoadError {
    ReadError,
    ParseError,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match *self {
            LoadError::ReadError => "read error",
            LoadError::ParseError => "parse error",
        };
        f.write_str(msg)
    }
}

impl Error for LoadError {}

pub fn load_tangent_buf(data: &[u8]) -> Result<(Vec<Vec3>, Vec<Vec3>), LoadError> {
    let reader = BufReader::new(data);

    let mut tangents: Vec<Vec3> = vec![];
    let mut bitangents: Vec<Vec3> = vec![];
    for line in reader.lines() {
        let (line, mut words) = match line {
            Ok(ref line) => (&line[..], line[..].split_whitespace()),
            Err(_e) => {
                #[cfg(feature = "log")]
                log::error!("load_tangent_buf - failed to read line due to {}", _e);
                return Err(LoadError::ReadError);
            }
        };
        match words.next() {
            Some("#") | None => continue,
            Some("t") => tangents.push(parse_vec3(words)?),
            Some("bt") => bitangents.push(parse_vec3(words)?),
            Some(_) => {}
        }
    }
    Ok((tangents, bitangents))
}

impl Mesh {
    fn new(
        gl: &Context,
        mesh_data: tobj::Mesh,
        material: Option<usize>,
        name: &str,
        tangent_loader: Option<&TangentLoader>,
        generate_tangents: bool,
    ) -> Self {
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

        let can_generate_tangets = generate_tangents && has_normals && has_texture_coordinates;

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
            tangent_buffer: if tangent_loader.is_some() || can_generate_tangets {
                unsafe { Some(gl.create_buffer().unwrap()) }
            } else {
                None
            },
            bitangent_buffer: if tangent_loader.is_some() || can_generate_tangets {
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

            if let Some(tangent_loader) = tangent_loader {
                let (tangents, bitangents) = load_tangent_buf(&tangent_loader(name)).unwrap();
                mesh.load_tangents(gl, &tangents, &bitangents);
            }

            if generate_tangents {
                if let Some(normals) = &normals {
                    if let Some(texture_coordinates) = &texture_coordinates {
                        mesh.generate_tangents(
                            gl,
                            &mesh_data.indices,
                            &mesh_data.positions,
                            normals,
                            texture_coordinates,
                        );
                    }
                }
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

    pub unsafe fn generate_tangents(
        &self,
        gl: &Context,
        indices: &[u32],
        positions: &[f32],
        normals: &[f32],
        texture_coordinates: &[f32],
    ) {
        let vertices: Vec<(Vec3, Vec3, Vec2)> = indices
            .iter()
            .map(|i| {
                let position = Vec3::new(
                    positions[(i * 3) as usize],
                    positions[(i * 3 + 1) as usize],
                    positions[(i * 3 + 2) as usize],
                );
                let normal = Vec3::new(
                    normals[(i * 3) as usize],
                    normals[(i * 3 + 1) as usize],
                    normals[(i * 3 + 2) as usize],
                );
                let tex_coord = Vec2::new(
                    texture_coordinates[(i * 2) as usize],
                    texture_coordinates[(i * 2 + 1) as usize],
                );
                (position, normal, tex_coord)
            })
            .collect();

        let mut tangents: Vec<Vec3> = vec![];
        let mut bitangents: Vec<Vec3> = vec![];

        for face in indices.chunks_exact(3) {
            let (p0, n0, t0) = vertices[face[0] as usize];
            let (p1, n1, t1) = vertices[face[1] as usize];
            let (p2, n2, t2) = vertices[face[2] as usize];

            let v = p1 - p0;
            let w = p2 - p0;

            let mut sx = t1.x - t0.x;
            let mut sy = t1.y - t0.y;
            let mut tx = t2.x - t0.x;
            let mut ty = t2.y - t0.y;
            let dir_correction = if tx * sy - ty * sx < 0.0 { -1.0 } else { 1.0 };

            if sx * ty == sy * tx {
                sx = 0.0;
                sy = 1.0;
                tx = 1.0;
                ty = 0.0;
            }

            let tangent = Vec3::new(
                (w.x * sy - v.x * ty) * dir_correction,
                (w.y * sy - v.y * ty) * dir_correction,
                (w.z * sy - v.z * ty) * dir_correction,
            );
            let bitangent = Vec3::new(
                (w.x * sx - v.x * tx) * dir_correction,
                (w.y * sx - v.y * tx) * dir_correction,
                (w.z * sx - v.z * tx) * dir_correction,
            );

            // Calculate local tangents and bitangents
            let local_tangent0 = (tangent - n0 * n0.dot(tangent)).normalize();
            let local_tangent1 = (tangent - n1 * n1.dot(tangent)).normalize();
            let local_tangent2 = (tangent - n2 * n2.dot(tangent)).normalize();

            let local_bitangent0 = (bitangent - n0 * n0.dot(bitangent)).normalize();
            let local_bitangent1 = (bitangent - n1 * n1.dot(bitangent)).normalize();
            let local_bitangent2 = (bitangent - n2 * n2.dot(bitangent)).normalize();
            tangents.push(reconstruct_tangent(local_tangent0, local_bitangent0, n0));
            tangents.push(reconstruct_tangent(local_tangent1, local_bitangent1, n1));
            tangents.push(reconstruct_tangent(local_tangent2, local_bitangent2, n2));
            bitangents.push(reconstruct_bitangent(local_tangent0, local_bitangent0, n0));
            bitangents.push(reconstruct_bitangent(local_tangent1, local_bitangent1, n1));
            bitangents.push(reconstruct_bitangent(local_tangent2, local_bitangent2, n2));
        }

        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_buffer(ARRAY_BUFFER, self.tangent_buffer);
        gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&tangents), STATIC_DRAW);

        gl.bind_buffer(ARRAY_BUFFER, self.bitangent_buffer);
        gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(&bitangents), STATIC_DRAW);
    }

    pub unsafe fn load_tangents(&self, gl: &Context, tangents: &[Vec3], bitangents: &[Vec3]) {
        if let Some(tangent_buffer) = self.tangent_buffer {
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(tangent_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(tangents), STATIC_DRAW);
        }
        if let Some(bitangent_buffer) = self.bitangent_buffer {
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(ARRAY_BUFFER, Some(bitangent_buffer));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, bytemuck::cast_slice(bitangents), STATIC_DRAW);
        }
    }

    pub fn bind(
        &self,
        gl: &Context,
        program: NativeProgram,
        vertex_binding: &str,
        normal_binding: Option<&str>,
        texture_binding: Option<&str>,
        tangent_binding: Option<&str>,
        bitangent_binding: Option<&str>,
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

            if let Some(tangent_binding) = tangent_binding {
                if let Some(loc) = gl.get_attrib_location(program, tangent_binding) {
                    gl.bind_buffer(ARRAY_BUFFER, Some(self.tangent_buffer.unwrap()));
                    gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(loc);
                }
            }
            if let Some(bitangent_binding) = bitangent_binding {
                if let Some(loc) = gl.get_attrib_location(program, bitangent_binding) {
                    gl.bind_buffer(ARRAY_BUFFER, Some(self.bitangent_buffer.unwrap()));
                    gl.vertex_attrib_pointer_f32(loc, 3, FLOAT, false, 0, 0);
                    gl.enable_vertex_attrib_array(loc);
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
        tangent_binding: Option<&str>,
        bitangent_binding: Option<&str>,
    ) {
        self.bind(
            gl,
            program,
            vertex_binding,
            normal_binding,
            texture_binding,
            tangent_binding,
            bitangent_binding,
        );
        unsafe { gl.draw_elements(TRIANGLES, self.num_indices as _, UNSIGNED_INT, 0) }
    }
}

#[derive(Debug, Clone, Copy)]
struct Material {
    ambient: Option<Vec3>,
    emissive: Option<Vec3>,
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
    pub emissive: Option<String>,
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
        let emissive = material.unknown_param.get("Ke").map(|word| {
            word.split_whitespace()
                .take(3)
                .map(FromStr::from_str)
                .collect::<Result<Vec<f32>, _>>()
                .map(|v| Vec3::from_slice(&v))
                .unwrap()
        });

        if let Some(tex_loader) = texture_loader {
            unsafe {
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

                let max_anisotropy = gl.get_parameter_f32(MAX_TEXTURE_MAX_ANISOTROPY_EXT);
                let normal_texture = material
                    .normal_texture
                    .map(|texture_name| tex_loader(&texture_name))
                    .map(|data| {
                        Texture::load_with_parameters(
                            gl,
                            &data,
                            &[
                                (TEXTURE_WRAP_S, REPEAT as _),
                                (TEXTURE_WRAP_R, REPEAT as _),
                                (TEXTURE_MIN_FILTER, LINEAR_MIPMAP_LINEAR as _),
                                (TEXTURE_MAG_FILTER, LINEAR as _),
                            ],
                            &[(TEXTURE_MAX_ANISOTROPY_EXT, max_anisotropy)],
                            true,
                        )
                    });
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
                    emissive,
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
            }
        } else {
            Material {
                ambient: material.ambient.map(Vec3::from_array),
                emissive,
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
            if let Some(emissive_binding) = &bindings.emissive {
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(program, &emissive_binding).as_ref(),
                    self.emissive.unwrap_or_default().as_ref(),
                );
                gl.uniform_1_i32(
                    gl.get_uniform_location(program, &format!("has_{}", emissive_binding))
                        .as_ref(),
                    self.emissive.is_some() as i32,
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
            tangent_buffer: None,
            bitangent_buffer: None,
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
        tangent_loader: Option<&TangentLoader>,
        generate_tangents: bool,
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
                Mesh::new(
                    gl,
                    mesh.mesh,
                    material_id,
                    &mesh.name,
                    tangent_loader,
                    generate_tangents,
                )
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
        tangent_binding: Option<&str>,
        bitangent_binding: Option<&str>,
        material_bindings: Option<&MaterialBindings>,
    ) {
        for mesh in &self.meshes {
            if let Some(material) = mesh.material {
                if let Some(m) = material_bindings {
                    self.material[material].bind(gl, program, m);
                }
            }
            mesh.draw(
                gl,
                program,
                vertex_binding,
                normal_binding,
                texture_binding,
                tangent_binding,
                bitangent_binding,
            );
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
        tangent_binding: Option<&str>,
        bitangent_binding: Option<&str>,
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
        mesh.draw(
            gl,
            program,
            vertex_binding,
            normal_binding,
            texture_binding,
            tangent_binding,
            bitangent_binding,
        );
    }
}
