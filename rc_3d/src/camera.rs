use std::f32::consts::PI;

use microglut::{
    glam::{Mat4, Vec3},
    imgui,
};

pub struct Camera {
    pub position: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
    pub walk_speed: f32,
    pub rotational_speed: f32,
    pitch: f32,
    yaw: f32,
}

impl Camera {
    pub fn new(
        position: Vec3,
        pitch: f32,
        yaw: f32,
        fov: f32,
        near: f32,
        far: f32,
        aspect_ratio: f32,
    ) -> Self {
        Self {
            position,
            pitch,
            yaw,
            fov,
            near,
            far,
            aspect_ratio,
            walk_speed: 1.0,
            rotational_speed: 1.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let look_x = self.yaw.cos() * self.pitch.cos();
        let look_y = self.pitch.sin();
        let look_z = self.yaw.sin() * self.pitch.cos();
        Vec3::new(look_x, look_y, look_z).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.forward().cross(self.right())
    }

    pub fn view_transform(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }

    pub fn perspective_transform(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn move_by(&mut self, relative_position: Vec3) {
        self.position += relative_position;
    }

    pub fn walk_forward(&mut self, amount: f32) {
        self.position += self.forward() * amount;
    }

    pub fn strafe(&mut self, amount: f32) {
        self.position += self.right() * amount;
    }

    pub fn fly(&mut self, amount: f32) {
        self.position += Vec3::Y * amount;
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
    }

    pub fn add_pitch(&mut self, pitch: f32) {
        self.set_pitch(self.pitch + pitch);
    }

    pub fn add_yaw(&mut self, yaw: f32) {
        self.set_yaw(self.yaw + yaw);
    }

    pub fn ui(&mut self, ui: &imgui::Ui) {
        if ui.tree_node("Camera").is_some() {
            ui.slider("Walk speed", 0.0, 10.0, &mut self.walk_speed);
            ui.slider("Rotational speed", 0.1, 10.0, &mut self.rotational_speed);
            ui.slider("FOV", 0.1, 1.9 * PI, &mut self.fov);
            ui.input_float3("Position", self.position.as_mut()).build();

            let mut pitch_deg = self.pitch.to_degrees();
            let mut yaw_deg = self.yaw.to_degrees();
            ui.input_float("Pitch", &mut pitch_deg).build();
            ui.input_float("Yaw", &mut yaw_deg).build();
            self.set_pitch(pitch_deg.to_radians());
            self.set_yaw(yaw_deg.to_radians());
        }
    }
}
