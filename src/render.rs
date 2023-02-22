use std::f32::INFINITY;
use std::sync::Arc;

use crate::camera::Camera;
use crate::hittable::{Bvh, Hittable};
use crate::material::Scatter;
use crate::ray::Ray;

use rayon::prelude::*;
use ultraviolet::Vec3;

#[inline]
fn ray_color(ray: Ray, world: Arc<Bvh>, depth: u32) -> Vec3 {
    // if depth == 0 {
    //     return Vec3::new(0.0, 0.0, 0.0);
    // }
    let mut color_total = Vec3::one();
    let mut temp_ray = ray;
    let mut bounce = 0;
    for _ in 0..depth {
        if let Some(hit) = world.hit(&temp_ray, 0.00015, INFINITY) {
            let scatter: Scatter = hit.material.scatter(temp_ray, hit);
            if scatter.attenuation.component_max() <= 1.0 + f32::EPSILON {
                color_total *= scatter.attenuation;
                temp_ray = scatter.ray;
            } else {
                color_total *= scatter.attenuation;
                break;
            }
        } else {
            let t = 0.5 * (ray.dir.y + 1.0);
            color_total *= (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0);
            break;
        }
        bounce += 1;
    }
    if bounce == depth {
        Vec3::zero()
    } else {
        color_total
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

pub fn render(
    width: usize,
    height: usize,
    camera: Camera,
    world: Arc<Bvh>,
    buffer: &[Vec3],
    sample_rate: u32,
    max_bounce: u32,
) -> Vec<Vec3> {
    (0..width * height)
        .into_par_iter()
        .chunks((width * height) / 64)
        .flat_map(|chunk| {
            chunk
                .iter()
                .map(|pixel| {
                    let y = height - 1 - pixel / width;
                    let x = pixel % width;
                    let mut pixel_color = Vec3::zero();
                    (0..sample_rate).for_each(|_| {
                        pixel_color += ray_color(camera.gen_ray(x, y), world.clone(), max_bounce);
                    });

                    buffer[*pixel] + (pixel_color / sample_rate as f32)
                })
                .collect::<Vec<Vec3>>()
        })
        .collect()
}

#[inline]
pub fn preview(width: usize, height: usize, camera: Camera, world: Arc<Bvh>) -> Vec<Vec3> {
    (0..width * height)
        .into_par_iter()
        .chunks((width * height) / 64)
        .flat_map(|chunk| {
            chunk
                .iter()
                .map(|pixel| {
                    color_only(
                        camera.gen_ray(pixel % width, height - 1 - pixel / width),
                        world.clone(),
                    )
                })
                .collect::<Vec<Vec3>>()
        })
        .collect()
}
