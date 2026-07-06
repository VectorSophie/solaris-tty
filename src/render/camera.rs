//! Free-fly camera: a position plus yaw/pitch look angles. This is the "flying
//! a probe" core. An orbit-around-target mode is a roadmap item.

use glam::{Mat4, Vec3};

pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,   // radians, around +Y
    pub pitch: f32, // radians, clamped to avoid gimbal flip
    pub fov_y_deg: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Start pulled back and above, looking at the origin (the Sun).
    pub fn looking_at_origin(pos: Vec3) -> Self {
        let fwd = (-pos).normalize_or_zero();
        let yaw = fwd.x.atan2(fwd.z);
        let pitch = fwd.y.asin();
        Camera {
            pos,
            yaw,
            pitch,
            fov_y_deg: 50.0,
            near: 0.01,
            far: 1000.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.pos, self.pos + self.forward(), Vec3::Y)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y_deg.to_radians(), aspect, self.near, self.far)
    }

    // --- controls; step scales with distance from origin so it feels right at
    // any zoom level. ---

    fn step(&self) -> f32 {
        (self.pos.length() * 0.06).clamp(0.15, 40.0)
    }

    pub fn move_forward(&mut self, sign: f32) {
        self.pos += self.forward() * self.step() * sign;
    }
    pub fn move_right(&mut self, sign: f32) {
        self.pos += self.right() * self.step() * sign;
    }
    pub fn move_up(&mut self, sign: f32) {
        self.pos += Vec3::Y * self.step() * sign;
    }
    pub fn turn(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-1.5, 1.5);
    }
}
