extern crate fastrand;
use crate::{ray::Ray, vec3::Vec3, ASPECT_RATIO, HEIGHT, WIDTH};

#[derive(Copy, Clone)]
pub struct Camera {
    pub eye: Vec3,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    lens_radius: f32,
}

impl Camera {
    pub fn new(
        origin: Vec3,
        lookat: Vec3,
        vup: Vec3,
        fov: f32,
        aspect_ratio: f32,
        apeture: f32,
        focus_dist: f32,
    ) -> Camera {
        // Camera setup
        let h = (fov.to_radians() / 2.0).tan();
        let viewport_height: f32 = 2.0 * h;
        let viewport_width: f32 = aspect_ratio * viewport_height;

        let w = (origin - lookat).normalize();
        let u = vup.cross(w).normalize();
        let v = w.cross(u);

        let horizontal = focus_dist * viewport_width * u;
        let vertical = focus_dist * viewport_height * v;
        let lower_left_corner = origin - (horizontal / 2.0) - (vertical / 2.0) - focus_dist * w;
        Camera {
            eye: origin,
            lower_left_corner,
            horizontal,
            vertical,
            u,
            v,
            w,
            lens_radius: apeture / 2.0,
        }
    }

    #[inline]
    pub fn gen_ray(&self, x: usize, y: usize) -> Ray {
        let rd: Vec3 = self.lens_radius * Vec3::random_in_unit_disk();
        let offset: Vec3 = rd.x * self.u + rd.y * self.v;

        let s = (x as f32 + fastrand::f32()) / (WIDTH - 1) as f32;
        let t = (y as f32 + fastrand::f32()) / (HEIGHT - 1) as f32;
        Ray::new(
            self.eye + offset,
            (self.lower_left_corner + (s * self.horizontal) + (t * self.vertical))
                - self.eye
                - offset,
        )
    }
}
