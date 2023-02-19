use std::f32::INFINITY;
use std::sync::Arc;

use crate::camera::Camera;
use crate::hittable::{Hittable, BVH};
use crate::material::Scatter;
use crate::ray::Ray;

use rayon::prelude::*;
use ultraviolet::Vec3;

fn ray_color(ray: Ray, world: Arc<BVH>, depth: u32) -> Vec3 {
    if depth == 0 {
        return Vec3::new(0.0, 0.0, 0.0);
    }

    if let Some(hit) = world.hit(&ray, 0.0002, INFINITY) {
        let scatter: Scatter = hit.material.scatter(ray, hit);
        if scatter.attenuation.mag_sq() <= 4.0 {
            scatter.attenuation * ray_color(scatter.ray, world, depth - 1)
        } else {
            scatter.attenuation
        }
    } else {
        let t = 0.5 * (ray.dir.y + 1.0);
        (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0)
    }
}

fn color_only(ray: Ray, world: Arc<BVH>) -> Vec3 {
    if let Some(hit) = world.hit(&ray, 0.0002, INFINITY) {
        (3.0 / hit.t) * hit.material.scatter(ray, hit).attenuation
    } else {
        let t = 0.5 * (ray.dir.y + 1.0);
        (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0)
    }
}

pub fn render(
    width: usize,
    height: usize,
    camera: Camera,
    world: Arc<BVH>,
    buffer: &[Vec3],
    sample_rate: u32,
    max_bounce: u32,
) -> Vec<Vec3> {
    (0..width * height)
        .into_par_iter()
        .map_init(
            || (),
            |_, screen_pos| {
                let y = height - 1 - screen_pos / width;
                let x = screen_pos % width;
                let mut pixel_color = Vec3::zero();
                (0..sample_rate).for_each(|_| {
                    pixel_color += ray_color(camera.gen_ray(x, y), world.clone(), max_bounce);
                });

                buffer[screen_pos] + (pixel_color / sample_rate as f32)
            },
        )
        .collect()
}

pub fn preview(width: usize, height: usize, camera: Camera, world: Arc<BVH>) -> Vec<Vec3> {
    (0..width * height)
        .into_par_iter()
        .map_init(
            || (),
            |_, screen_pos| {
                color_only(
                    camera.gen_ray(screen_pos % width, height - 1 - screen_pos / width),
                    world.clone(),
                )
            },
        )
        .collect()
}
