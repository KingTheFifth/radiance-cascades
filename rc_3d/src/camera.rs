use microglut::glam::{Mat4, Quat, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub look_direction: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
}

impl Camera {
    pub fn new(
        position: Vec3,
        look_direction: Vec3,
        fov: f32,
        near: f32,
        far: f32,
        aspect_ratio: f32,
    ) -> Self {
        Self {
            position,
            look_direction,
            fov,
            near,
            far,
            aspect_ratio,
        }
    }

    pub fn forward(&self) -> Vec3 {
        (-self.look_direction).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.look_direction.cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.forward().cross(self.right())
    }

    pub fn view_transform(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.look_direction, Vec3::Y)
    }

    pub fn perspective_transform(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    pub fn move_by(&mut self, relative_position: Vec3) {
        self.position += relative_position;
    }

    pub fn rotate(&mut self, rotation: Quat) {
        let rot_mat = Mat4::from_quat(rotation);
        self.look_direction = rot_mat.transform_vector3(self.look_direction);
    }
}
