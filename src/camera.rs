extern crate fastrand;
use crate::{ray::Ray, vec3::Vec3, ASPECT_RATIO, HEIGHT, WIDTH};

#[derive(Copy, Clone)]
pub struct Camera {
    pub eye: Vec3,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl Camera {
    pub fn new(origin: Vec3, focal_length: f32) -> Camera {
        // Camera setup
        let viewport_height: f32 = 2.0;
        let viewport_width: f32 = ASPECT_RATIO * viewport_height;

        let horizontal = Vec3::new(viewport_width, 0.0, 0.0);
        let vertical = Vec3::new(0.0, viewport_height, 0.0);
        let lower_left_corner =
            origin - (horizontal / 2.0) - (vertical / 2.0) - Vec3::new(0.0, 0.0, focal_length);
        Camera {
            eye: origin,
            lower_left_corner,
            horizontal,
            vertical,
        }
    }

    #[inline]
    pub fn gen_ray(&self, x: usize, y: usize) -> Ray {
        let u = (x as f32 + fastrand::f32()) / (WIDTH - 1) as f32;
        let v = (y as f32 + fastrand::f32()) / (HEIGHT - 1) as f32;
        Ray::new(
            self.eye,
            (self.lower_left_corner + (u * self.horizontal) + (v * self.vertical)) - self.eye,
        )
    }
}
