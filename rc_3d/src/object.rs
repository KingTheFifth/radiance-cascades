use microglut::{
    glam::{Mat4, Quat, Vec3, Vec4},
    Model,
};

pub struct Object {
    pub model: Model,
    rotation: Quat,
    translation: Vec3,
    scale: Vec3,
    pub albedo: Vec4,
    pub emissive: Vec4,
}

impl Object {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            rotation: Quat::IDENTITY,
            translation: Vec3::ZERO,
            scale: Vec3::ONE,
            albedo: Vec4::ONE,
            emissive: Vec4::ZERO,
        }
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_uniform_scale(self, scale: f32) -> Self {
        self.with_scale(Vec3::new(scale, scale, scale))
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn get_transformation(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn with_albedo(mut self, albedo: Vec4) -> Self {
        self.albedo = albedo;
        self
    }

    pub fn with_emissive(mut self, emissive: Vec4) -> Self {
        self.emissive = emissive;
        self
    }
}
