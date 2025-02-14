use microglut::glam::{Mat4, Quat, Vec2, Vec3};

pub struct Sprite {
    position: Vec2,
    pub texture_index: u32,
    pub model_to_world: Mat4,
}

impl Sprite {
    pub fn new(texture_index: u32, position: Vec2, scale: Vec2, rotation: f32) -> Sprite {
        Sprite {
            position: position,
            texture_index: texture_index,
            model_to_world: Mat4::from_scale_rotation_translation(
                Vec3::new(scale.x, scale.y, 1.0),
                Quat::from_rotation_z(rotation),
                Vec3::new(position.x, position.y, 0.0),
            ),
        }
    }
}
