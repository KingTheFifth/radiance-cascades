use std::f32::consts::PI;

use microglut::{
    glam::{Mat4, Quat, Vec3},
    imgui,
};

pub struct Camera {
    pub position: Vec3,
    pub look_direction: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
    pub walk_speed: f32,
    pub rotational_speed: f32,
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
            walk_speed: 1.0,
            rotational_speed: 1.0,
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

    pub fn ui(&mut self, ui: &imgui::Ui) {
        if ui.tree_node("Camera").is_some() {
            ui.slider("Walk speed", 0.0, 10.0, &mut self.walk_speed);
            ui.slider("Rotational speed", 0.1, 10.0, &mut self.rotational_speed);
            ui.slider("FOV", 0.1, 1.9 * PI, &mut self.fov);
        }
    }
}
