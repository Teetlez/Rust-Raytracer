use std::f32::consts::PI;
use std::f32::INFINITY;
use std::sync::Arc;

use crate::camera::Camera;
use crate::hittable::{Bvh, Hittable};
use crate::material::Scatter;
use crate::ray::Ray;
use quasirandom::Qrng;
use radiant::Image;
use rayon::prelude::*;
use ultraviolet::{Mat3, Vec3};

const T_MIN: f32 = 0.00015;
const T_MAX: f32 = 100000.0;

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
fn ray_color(
    ray: Ray,
    world: Arc<Bvh>,
    depth: u32,
    image: Arc<Option<Image>>,
    light_clamp: f32,
) -> Vec3 {
    let mut color_total = Vec3::one();
    let mut temp_ray = ray;
    for _ in 0..depth {
        if let Some(hit) = world.hit(&temp_ray, T_MIN, T_MAX) {
            let scatter: Scatter =
                hit.material
                    .scatter(temp_ray, hit, fastrand::f32(), fastrand::f32());
            if scatter.attenuation.component_max() <= 1.0 {
                color_total *= scatter.attenuation;
                if color_total.component_max() < fastrand::f32() {
                    break;
                }
                color_total *= color_total.component_max().recip();
                temp_ray = scatter.ray;
            } else {
                return color_total
                    * scatter
                        .attenuation
                        .clamped(Vec3::zero(), Vec3::one() * light_clamp);
            }
        } else {
            return color_total * get_sky(temp_ray, image, light_clamp);
        }
    }
    color_total * 0.01
}

#[inline]
fn get_pixel_from_vec(dir: Vec3, image: Arc<Option<Image>>) -> Option<Vec3> {
    if let Some(img) = image.as_ref() {
        let u = (dir.x.atan2(dir.z) + PI) / (2.0 * PI);
        let v = (-dir.y).acos() / PI;

        if u <= 1.0 && v <= 1.0 {
            let color = img.pixel(
                (u * img.width as f32) as usize,
                ((1.0 - v) * img.height as f32) as usize,
            );
            Some(Vec3::new(color.r, color.g, color.b))
        } else {
            None
        }
    } else {
        None
    }
}

#[inline]
fn color_only(ray: Ray, world: Arc<Bvh>, image: Arc<Option<Image>>) -> Vec3 {
    if let Some(hit) = world.hit(&ray, T_MIN, T_MAX) {
        (Vec3::new(1.0, 1.0, -0.5))
            .normalized()
            .dot(hit.normal)
            .clamp(0.1, 1.0)
            * hit
                .material
                .scatter(ray, hit, fastrand::f32(), fastrand::f32())
                .attenuation
    } else {
        get_sky(ray, image, INFINITY)
    }
}

#[inline]
fn get_sky(ray: Ray, image: Arc<Option<Image>>, light_clamp: f32) -> Vec3 {
    if let Some(color) = get_pixel_from_vec(ray.dir, image) {
        Vec3::new(color[0], color[1], color[2]).clamped(Vec3::zero(), Vec3::one() * light_clamp)
    } else {
        let t = 0.5 * (ray.dir.dot(Vec3::new(-1.0, 0.75, 0.5).normalized()) + 1.0);
        ((1.0 - t) * Vec3::one() + t * Vec3::new(0.1, 0.3, 0.8)) * 2.0
    }
}

#[derive(Clone)]
pub struct Renderer {
    pub width: usize,
    pub height: usize,
    pub camera: Camera,
    pub world: Arc<Bvh>,
    pub sample_rate: u32,
    pub max_bounce: u32,
    pub hdr: Arc<Option<Image>>,
    pub light_clamp: f32,
}
impl Renderer {
    pub fn render(&self, buffer: &[Vec3]) -> Vec<Vec3> {
        (0..self.width * self.height)
            .into_par_iter()
            .chunks((self.width * self.height) / 64)
            .flat_map(|chunk| {
                let hdr = self.hdr.clone();
                let qrng = &mut Qrng::<(f32, f32)>::new(fastrand::f64());
                let sample_vec = (0..(chunk.len() * self.sample_rate as usize))
                    .map(|_| qrng.gen())
                    .collect::<Vec<(f32, f32)>>();
                chunk
                    .iter()
                    .map(|pixel| {
                        let x = (pixel % self.width) as f32;
                        let y = (self.height - 1 - pixel / self.width) as f32;
                        let mut pixel_color = Vec3::zero();
                        let mut offset = fastrand::usize(0..sample_vec.len());
                        (0..self.sample_rate).for_each(|_| {
                            let (jx, jy) = sample_vec[offset % sample_vec.len()];
                            offset += 1;

                            pixel_color += ray_color(
                                self.camera.gen_ray(self.width, self.height, x, y, jx, jy),
                                self.world.clone(),
                                self.max_bounce,
                                hdr.clone(),
                                self.light_clamp,
                            );
                            if !pixel_color.x.is_finite() {
                                pixel_color.x = 0.0;
                            }
                            if !pixel_color.y.is_finite() {
                                pixel_color.y = 0.0;
                            }
                            if !pixel_color.z.is_finite() {
                                pixel_color.z = 0.0;
                            }
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
                let qrng = &mut Qrng::<(f32, f32)>::new(fastrand::f64());
                let sample_vec = (0..(chunk.len() as usize))
                    .map(|_| qrng.gen())
                    .collect::<Vec<(f32, f32)>>();
                let mut offset = fastrand::usize(0..sample_vec.len());
                chunk
                    .iter()
                    .map(|pixel| {
                        offset += 1;
                        let (jx, jy) = sample_vec[offset % sample_vec.len()];
                        color_only(
                            self.camera.gen_ray(
                                self.width,
                                self.height,
                                (pixel % self.width) as f32,
                                (self.height - 1 - pixel / self.width) as f32,
                                jx,
                                jy,
                            ),
                            self.world.clone(),
                            self.hdr.clone(),
                        )
                    })
                    .collect::<Vec<Vec3>>()
            })
            .collect()
    }
}
