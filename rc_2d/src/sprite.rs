use microglut::glam::{Mat4, Quat, Vec2, Vec3};

pub struct Sprite {
    position: Vec2,
    texture_index: u32,
    pub model_to_world: Mat4,
}

impl Sprite {
    pub fn new(texture_index: u32, position: Vec2, scale: Vec2, rotation: f32) -> Sprite {
        Sprite {
            position: position,
            texture_index: texture_index,
            model_to_world: Mat4::from_scale_rotation_translation(
                Vec3::new(scale.x, 0.0, scale.y),
                Quat::from_rotation_y(rotation),
                Vec3::new(position.x, 0.0, position.y),
            ),
        }
    }
}
