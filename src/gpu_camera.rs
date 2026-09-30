use bytemuck::{Pod, Zeroable};

use ultraviolet::{Vec2, Vec3, Vec4};
const SENSETIVITY: f32 = 0.001;
const MAX_PITCH: f32 = 1.553343;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub enum Direction {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct CameraUniforms {
    eye: Vec3, // origin of the camera
    _pad0: u32,
    uvw: [Vec4; 3], // (u, v, w camera coordinates)
    hvc: [Vec4; 3], // (horizontal, vertical, corner viewport coords)
    lens_rd: Vec2,  // (focus radius, focus distance)
    _pad1: Vec2,
}

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    uniforms: CameraUniforms,
    eye: Vec3,
    lookat: Vec3,
    yaw: f32,
    pitch: f32,
    vup: Vec3,
    fov: f32,
    aspect_ratio: f32,
    apeture: f32,
    focus_dist: f32,
}

impl Camera {
    pub fn new(
        eye: Vec3,
        lookat: Vec3,
        vup: Vec3,
        fov: f32,
        aspect_ratio: f32,
        apeture: f32,
        focus_dist: f32,
    ) -> Camera {
        let lookat = lookat.normalized();
        Camera {
            uniforms: CameraUniforms::zeroed(),
            eye,
            lookat,
            yaw: lookat.x.atan2(-lookat.z),
            pitch: lookat.y.asin(),
            vup,
            fov,
            aspect_ratio,
            apeture,
            focus_dist,
        }
    }

    pub fn uniforms(&self) -> &CameraUniforms {
        &self.uniforms
    }

    pub fn update_uniforms(&mut self) {
        // Camera setup
        let h = (self.fov.to_radians() / 2.0).tan();
        let viewport_height: f32 = 2.0 * h;
        let viewport_width: f32 = self.aspect_ratio * viewport_height;

        let w = -self.lookat.normalized();
        let u = self.vup.cross(w).normalized();
        let v = w.cross(u);

        let horizontal = self.focus_dist * viewport_width * u;
        let vertical = self.focus_dist * viewport_height * v;
        let lower_left_corner =
            self.eye - (horizontal / 2.0) - (vertical / 2.0) - self.focus_dist * w;
        self.uniforms.eye = self.eye;
        self.uniforms.uvw = [u.into(), v.into(), w.into()];
        self.uniforms.hvc = [horizontal.into(), vertical.into(), lower_left_corner.into()];
        self.uniforms.lens_rd = Vec2::new(self.apeture / 2.0, self.focus_dist);
    }

    pub fn translate(&mut self, dir: Direction, speed: f32) {
        let (sign, axis) = match dir {
            Direction::Left => (-1.0, 0),
            Direction::Right => (1.0, 0),
            Direction::Down => (-1.0, 1),
            Direction::Up => (1.0, 1),
            Direction::Forward => (-1.0, 2),
            Direction::Backward => (1.0, 2),
        };
        let delta = (sign * self.uniforms.uvw[axis] * speed).truncated();
        self.eye += delta;
    }

    pub fn update_lookat(&mut self, du: f32, dv: f32) {
        self.yaw += du * SENSETIVITY;
        self.pitch = (self.pitch - dv * SENSETIVITY).clamp(-MAX_PITCH, MAX_PITCH);
        let cos_pitch = self.pitch.cos();
        self.lookat = Vec3::new(
            self.yaw.sin() * cos_pitch,
            self.pitch.sin(),
            -self.yaw.cos() * cos_pitch,
        );
    }

    pub fn zoom(&mut self, delta: f32, scale: f32) {
        self.fov += delta * 0.833_333_4 * scale;
    }

    pub fn focus(&mut self, delta: f32, scale: f32) {
        self.focus_dist += delta * scale;
    }

    pub fn apeture(&mut self, delta: f32, scale: f32) {
        self.apeture += delta * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_mouse_motion_changes_pitch_not_yaw() {
        let mut camera = Camera::new(
            Vec3::zero(),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::unit_y(),
            60.0,
            1.0,
            0.0,
            1.0,
        );

        camera.update_lookat(0.0, 100.0);

        assert!(camera.lookat.y < 0.0);
        assert!(camera.lookat.x.abs() < 1e-6);
        assert!(camera.lookat.z < 0.0);
    }

    #[test]
    fn horizontal_mouse_motion_changes_yaw_not_pitch() {
        let mut camera = Camera::new(
            Vec3::zero(),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::unit_y(),
            60.0,
            1.0,
            0.0,
            1.0,
        );

        camera.update_lookat(100.0, 0.0);

        assert!(camera.lookat.x > 0.0);
        assert!(camera.lookat.y.abs() < 1e-6);
        assert!(camera.lookat.z < 0.0);
    }
}
