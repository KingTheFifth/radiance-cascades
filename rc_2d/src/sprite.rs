use bytemuck::{Pod, Zeroable};
use microglut::glam::{Mat4, Quat, Vec2, Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Sprite {
    pub model_to_world: Mat4,
    pub albedo: Vec4,
    pub emissive: Vec4,
    pub texture_index: f32,
    _padding: [f32; 3],
}

impl Sprite {
    pub fn new(texture_index: f32, position: Vec2, scale: Vec2, rotation: f32) -> Sprite {
        Sprite {
            texture_index: texture_index,
            model_to_world: Mat4::from_scale_rotation_translation(
                Vec3::new(scale.x, scale.y, 1.0),
                Quat::from_rotation_z(rotation),
                Vec3::new(position.x, position.y, 0.0),
            ),
            albedo: Vec4::ZERO,
            emissive: Vec4::ONE,
            _padding: Default::default(),
        }
    }
}
