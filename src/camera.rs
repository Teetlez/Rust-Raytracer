extern crate fastrand;
use crate::{random, ray::Ray};

use ultraviolet::{Rotor3, Vec3};

const SENSETIVITY: f32 = 0.0025;

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    view: (Vec3, Vec3, f32),
    aspect_ratio: f32,
    hvc: [Vec3; 3],
    uvw: [Vec3; 3],
    lens_rd: (f32, f32),
    mouse: Option<(f32, f32)>,
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
            view: (eye, lookat, fov),
            aspect_ratio,
            hvc: [horizontal, vertical, lower_left_corner],
            uvw: [u, v, w],
            lens_rd: (apeture / 2.0, focus_dist),
            mouse: None,
        }
    }

    #[inline]
    pub fn gen_ray(&self, width: usize, height: usize, x: f32, y: f32, jx: f32, jy: f32) -> Ray {
        let rd: Vec3 = self.lens_rd.0 * random::quasirandom_in_cocentric_disk(jx, jy);
        let offset: Vec3 = rd.x * self.uvw[0] + rd.y * self.uvw[1];

        let s = (x + ((jy * 0.5) - 1.0)) / (width - 1) as f32;
        let t = (y + ((jx * 0.5) - 1.0)) / (height - 1) as f32;
        Ray::new(
            self.view.0 + offset,
            (self.hvc[2] + (s * self.hvc[0]) + (t * self.hvc[1])) - self.view.0 - offset,
        )
    }

    pub fn reset_mouse(&mut self) {
        self.mouse = None;
    }

    pub fn update_lookat(&mut self, mouse_move: Option<(f32, f32)>) {
        if let Some((x, y)) = mouse_move {
            if let Some((mouse_x, mouse_y)) = self.mouse {
                let (diff_x, diff_y) = (mouse_x - x, y - mouse_y);
                let turn =
                    Rotor3::from_euler_angles(0.0, diff_y * SENSETIVITY, diff_x * SENSETIVITY);
                let new_view = Vec3::unit_z().rotated_by(turn);

                // self.view.1 = (self.view.1 - self.view.0).rotated_by(turn) + self.view.0;

                let w = (new_view.x * self.uvw[0]
                    + new_view.y * self.uvw[1]
                    + new_view.z * self.uvw[2])
                    .normalized();
                let u = Vec3::unit_y().cross(w).normalized();

                self.uvw = [u, w.cross(u), w];
            }
            self.mouse = Some((x, y));
        }
    }

    pub fn update(&mut self, keys: Vec<minifb::Key>, mouse_move: Option<(f32, f32)>, scroll: f32) {
        let h = (self.view.2.to_radians() / 2.0).tan();
        let viewport_height: f32 = 2.0 * h;
        let viewport_width: f32 = self.aspect_ratio * viewport_height;

        if scroll != 0.0 {
            self.lens_rd.1 += scroll * 0.083_333_336;
        }

        self.update_lookat(mouse_move);

        keys.iter().for_each(|key| match key {
            minifb::Key::W => self.forward(),
            minifb::Key::A => self.left(),
            minifb::Key::S => self.back(),
            minifb::Key::D => self.right(),
            minifb::Key::E => self.up(),
            minifb::Key::Q => self.down(),
            minifb::Key::F => self.lens_rd.0 = (self.lens_rd.0 - 0.0025).max(0.0),
            minifb::Key::R => self.lens_rd.0 += 0.0025,
            _ => (),
        });

        self.hvc[0] = self.lens_rd.1 * viewport_width * self.uvw[0];
        self.hvc[1] = self.lens_rd.1 * viewport_height * self.uvw[1];
        self.hvc[2] =
            self.view.0 - (self.hvc[0] / 2.0) - (self.hvc[1] / 2.0) - self.lens_rd.1 * self.uvw[2];
    }

    fn forward(&mut self) {
        self.view.0 -= self.uvw[2] * 0.5;
        self.view.1 -= self.uvw[2] * 0.5;
    }

    fn left(&mut self) {
        self.view.0 -= self.uvw[0] * 0.5;
        self.view.1 -= self.uvw[0] * 0.5;
    }

    fn back(&mut self) {
        self.view.0 += self.uvw[2] * 0.5;
        self.view.1 += self.uvw[2] * 0.5;
    }

    fn right(&mut self) {
        self.view.0 += self.uvw[0] * 0.5;
        self.view.1 += self.uvw[0] * 0.5;
    }

    fn up(&mut self) {
        self.view.0 += self.uvw[1] * 0.5;
        self.view.1 += self.uvw[1] * 0.5;
    }

    fn down(&mut self) {
        self.view.0 -= self.uvw[1] * 0.5;
        self.view.1 -= self.uvw[1] * 0.5;
    }
}
