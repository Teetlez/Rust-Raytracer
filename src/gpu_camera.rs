use {
    bytemuck::{Pod, Zeroable},
    std::f32::consts::{FRAC_PI_2, PI},
};

use ultraviolet::{Rotor3, Vec2, Vec3, Vec4, Vec4x4};
const SENSETIVITY: f32 = 0.001;

#[derive(Debug, Copy, Clone)]
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
        Camera {
            uniforms: CameraUniforms::zeroed(),
            eye,
            lookat,
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

        let w = self.lookat.normalized();
        let u = self.vup.cross(w).normalized();
        let v = w.cross(u);

        let horizontal = self.focus_dist * viewport_width * u;
        let vertical = self.focus_dist * viewport_height * v;
        let lower_left_corner =
            self.eye - (horizontal / 2.0) - (vertical / 2.0) - self.focus_dist * w;
        self.uniforms.eye = self.eye.into();
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
        let turn = Rotor3::from_euler_angles(0.0, -dv * SENSETIVITY, -du * SENSETIVITY);
        self.lookat = (self.lookat.rotated_by(turn)).normalized();
    }

    pub fn zoom(&mut self, delta: f32, scale: f32) {
        self.fov += delta * 0.83333336 * scale;
    }

    pub fn focus(&mut self, delta: f32, scale: f32) {
        self.focus_dist += delta * scale;
    }

    pub fn apeture(&mut self, delta: f32, scale: f32) {
        self.apeture += delta * scale;
    }
}
