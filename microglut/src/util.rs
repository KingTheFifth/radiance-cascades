use glam::{Mat4, Vec3};

pub fn arb_rotate(axis: Vec3, fi: f32) -> Mat4 {
    let eps = 0.000001;
    if axis.x.abs() < eps && axis.y.abs() < eps {
        // parallel to z
        if axis.z > 0.0 {
            Mat4::from_rotation_z(fi)
        } else {
            Mat4::from_rotation_z(-fi)
        }
    } else {
        let x = axis.normalize();
        let y = x.cross(Vec3::Z).normalize();
        let z = x.cross(y);
        let r = Mat4::from_cols_array_2d(&[
            [x.x, x.y, x.z, 0.0],
            [y.x, y.y, y.z, 0.0],
            [z.x, z.y, z.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let r_inv = r.transpose();
        let r_axel = Mat4::from_rotation_x(fi);
        (r_inv * r_axel) * r
    }
}
