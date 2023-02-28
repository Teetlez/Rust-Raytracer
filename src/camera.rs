extern crate fastrand;
use crate::{random, ray::Ray};

use ultraviolet::Vec3;

#[derive(Copy, Clone)]
pub struct Camera {
    pub eye: Vec3,
    lookat: Vec3,
    fov: f32,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    uvw: [Vec3; 3],
    lens_radius: f32,
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
        // Camera setup
        let h = (fov.to_radians() / 2.0).tan();
        let viewport_height: f32 = 2.0 * h;
        let viewport_width: f32 = aspect_ratio * viewport_height;

        let w = (eye - lookat).normalized();
        let u = vup.cross(w).normalized();
        let v = w.cross(u);

        let horizontal = focus_dist * viewport_width * u;
        let vertical = focus_dist * viewport_height * v;
        let lower_left_corner = eye - (horizontal / 2.0) - (vertical / 2.0) - focus_dist * w;
        Camera {
            eye,
            lookat,
            fov,
            lower_left_corner,
            horizontal,
            vertical,
            uvw: [u, v, w],
            lens_radius: apeture / 2.0,
            focus_dist,
        }
    }

    #[inline]
    pub fn gen_ray(&self, width: usize, height: usize, x: f32, y: f32) -> Ray {
        let rd: Vec3 = self.lens_radius * random::random_in_unit_disk();
        let offset: Vec3 = rd.x * self.uvw[0] + rd.y * self.uvw[1];

        let s = x / (width - 1) as f32;
        let t = y / (height - 1) as f32;
        Ray::new(
            self.eye + offset,
            (self.lower_left_corner + (s * self.horizontal) + (t * self.vertical))
                - self.eye
                - offset,
        )
    }

    pub fn update_lookat(&mut self, _mouse_move: Option<(f32, f32)>) {}

    pub fn update(&mut self, keys: Vec<minifb::Key>, mouse_move: Option<(f32, f32)>, scroll: f32) {
        let h = (self.fov.to_radians() / 2.0).tan();
        let viewport_height: f32 = 2.0 * h;
        let viewport_width: f32 = (3.0 / 2.0) * viewport_height;

        if scroll != 0.0 {
            self.focus_dist += scroll / 12.0;
        }

        self.update_lookat(mouse_move);

        keys.iter().for_each(|key| match key {
            minifb::Key::W => self.forward(),
            minifb::Key::A => self.left(),
            minifb::Key::S => self.back(),
            minifb::Key::D => self.right(),
            minifb::Key::Q => self.roll_left(),
            minifb::Key::E => self.roll_right(),
            minifb::Key::LeftShift => self.up(),
            minifb::Key::LeftCtrl => self.down(),
            minifb::Key::F => self.lens_radius -= 0.05,
            minifb::Key::R => self.lens_radius += 0.05,
            _ => (),
        });

        self.horizontal = self.focus_dist * viewport_width * self.uvw[0];
        self.vertical = self.focus_dist * viewport_height * self.uvw[1];
        self.lower_left_corner = self.eye
            - (self.horizontal / 2.0)
            - (self.vertical / 2.0)
            - self.focus_dist * self.uvw[2];
    }

    fn forward(&mut self) {
        self.eye -= self.uvw[2] * 0.5;
        self.lookat -= self.uvw[2] * 0.5;
    }

    fn left(&mut self) {
        todo!()
    }

    fn back(&mut self) {
        self.eye += self.uvw[2] * 0.5;
        self.lookat += self.uvw[2] * 0.5;
    }

    fn right(&mut self) {
        todo!()
    }

    fn roll_left(&mut self) {
        todo!()
    }

    fn roll_right(&mut self) {
        todo!()
    }

    fn up(&mut self) {
        let vup = Vec3::new(0.0, 0.1, 0.0);
        self.eye += vup;
        self.lookat += vup;
    }

    fn down(&mut self) {
        let vup = Vec3::new(0.0, 0.1, 0.0);
        self.eye -= vup;
        self.lookat -= vup;
    }
}
