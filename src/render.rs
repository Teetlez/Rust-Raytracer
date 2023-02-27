use std::f32::consts::PI;
use std::f32::INFINITY;
use std::sync::Arc;

use crate::camera::Camera;
use crate::hittable::{Bvh, Hittable};
use crate::material::Scatter;
use crate::ray::Ray;
use radiant::Image;
use rayon::prelude::*;
use ultraviolet::{Mat3, Vec3};

// Tonemapping constants
const M1: Mat3 = Mat3::new(
    Vec3::new(0.59719, 0.07600, 0.02840),
    Vec3::new(0.35458, 0.90834, 0.13383),
    Vec3::new(0.04823, 0.01566, 0.83777),
);
const M2: Mat3 = Mat3::new(
    Vec3::new(1.60475, -0.10208, -0.00327),
    Vec3::new(-0.53108, 1.10813, -0.07276),
    Vec3::new(-0.07367, -0.00605, 1.07602),
);

#[inline]
pub fn to_rgb(color: &Vec3, gamma: f32) -> u32 {
    let out = aces_tonemap(color, gamma);
    255 << 24
        | ((out.x * 255.4) as u32) << 16
        | ((out.y * 255.4) as u32) << 8
        | ((out.z * 255.4) as u32)
}

#[inline]
fn aces_tonemap(color: &Vec3, gamma: f32) -> Vec3 {
    let v = M1 * (*color);
    let a = v * (v + (Vec3::one() * 0.0245786)) - (Vec3::one() * 0.000090537);
    let b = v * (0.983729 * v + (Vec3::one() * 0.432951)) + (Vec3::one() * 0.238081);
    (M2 * (a / b))
        .clamped(Vec3::zero(), Vec3::one())
        .map(|c| c.powf(gamma))
}

#[inline]
fn ray_color(ray: Ray, world: Arc<Bvh>, depth: u32, image: Arc<Image>) -> Vec3 {
    // if depth == 0 {
    //     return Vec3::new(0.0, 0.0, 0.0);
    // }
    let mut color_total = Vec3::one();
    let mut temp_ray = ray;
    for _ in 0..depth {
        if let Some(hit) = world.hit(&temp_ray, 0.00015, INFINITY) {
            let scatter: Scatter = hit.material.scatter(temp_ray, hit);
            if scatter.attenuation.component_max() <= 1.0 + f32::EPSILON {
                color_total *= scatter.attenuation;
                temp_ray = scatter.ray;
            } else {
                return color_total * scatter.attenuation;
            }
        } else {
            return if let Some(color) = get_pixel_from_vec(temp_ray.dir, image) {
                color_total * Vec3::new(color[0], color[1], color[2])
                //.clamped(Vec3::zero(), Vec3::one() * 64.0)
            } else {
                let t = 0.5 * (temp_ray.dir.y + 1.0);
                color_total * ((1.0 - t) * Vec3::one() + t * Vec3::new(0.5, 0.7, 1.0)) * 1.5
            };
        }
    }
    color_total * 0.01
}

#[inline]
fn get_pixel_from_vec(dir: Vec3, image: Arc<Image>) -> Option<Vec3> {
    let u = (dir.x.atan2(dir.z) + PI) / (2.0 * PI);
    let v = (-dir.y).acos() / PI;

    if u <= 1.0 && v <= 1.0 {
        let color = image.pixel(
            (u * image.width as f32) as usize,
            ((1.0 - v) * image.height as f32) as usize,
        );
        Some(Vec3::new(color.r, color.g, color.b))
    } else {
        None
    }
}

fn color_only(ray: Ray, world: Arc<Bvh>) -> Vec3 {
    if let Some(hit) = world.hit(&ray, 0.0002, 100.0) {
        ((2.0 + Vec3::unit_y().dot(hit.normal)) / hit.t)
            * hit.material.scatter(ray, hit).attenuation
    } else {
        let t = 0.5 * (ray.dir.y + 1.0);
        (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0)
    }
}

pub struct Renderer {
    pub width: usize,
    pub height: usize,
    pub camera: Camera,
    pub world: Arc<Bvh>,
    pub sample_rate: u32,
    pub max_bounce: u32,
    pub hdr: Arc<Image>,
}
impl Renderer {
    pub fn render(&self, buffer: &[Vec3]) -> Vec<Vec3> {
        (0..self.width * self.height)
            .into_par_iter()
            .chunks((self.width * self.height) / 64)
            .flat_map(|chunk| {
                let hdr = self.hdr.clone();
                chunk
                    .iter()
                    .map(|pixel| {
                        let y = self.height - 1 - pixel / self.width;
                        let x = pixel % self.width;
                        let mut pixel_color = Vec3::zero();
                        (0..self.sample_rate).for_each(|_| {
                            pixel_color += ray_color(
                                self.camera.gen_ray(self.width, self.height, x, y),
                                self.world.clone(),
                                self.max_bounce,
                                hdr.clone(),
                            );
                        });

                        buffer[*pixel] + (pixel_color / self.sample_rate as f32)
                    })
                    .collect::<Vec<Vec3>>()
            })
            .collect()
    }

    #[inline]
    pub fn preview(&self) -> Vec<Vec3> {
        (0..self.width * self.height)
            .into_par_iter()
            .chunks((self.width * self.height) / 64)
            .flat_map(|chunk| {
                chunk
                    .iter()
                    .map(|pixel| {
                        color_only(
                            self.camera.gen_ray(
                                self.width,
                                self.height,
                                pixel % self.width,
                                self.height - 1 - pixel / self.width,
                            ),
                            self.world.clone(),
                        )
                    })
                    .collect::<Vec<Vec3>>()
            })
            .collect()
    }
}
