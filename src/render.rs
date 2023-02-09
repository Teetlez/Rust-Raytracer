use std::f32::INFINITY;

use crate::camera::Camera;
use crate::hittable::World;
use crate::material::Scatter;
use crate::ray::Ray;
use crate::{MAX_BOUNCE, SAMPLES};
use minifb::clamp;
use rayon::prelude::*;

use crate::vec3::Vec3;

fn ray_color(ray: Ray, world: &World, depth: usize) -> Vec3 {
    if depth >= MAX_BOUNCE {
        return Vec3::new(0.0, 0.0, 0.0);
    }

    if let Some(hit) = world.hit(&ray, 0.0005, INFINITY) {
        let scatter: Scatter = hit.material.scatter(ray, hit);
        if scatter.attenuation.length_sq() <= 3.0 {
            scatter.attenuation * ray_color(scatter.ray, world, depth + 1)
        } else {
            scatter.attenuation
        }
    } else {
        let t = 0.5 * (ray.dir.y + 1.0);
        (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0)
    }
}

pub fn render(
    width: usize,
    height: usize,
    camera: Camera,
    world: &World,
    buffer: &Vec<Vec3>,
) -> Vec<Vec3> {
    (0..width * height)
        .into_par_iter()
        .map_init(
            || (),
            |_, screen_pos| {
                let y = height - 1 - screen_pos / width;
                let x = screen_pos % width;
                let mut pixel_color = Vec3::zero();
                (0..SAMPLES).for_each(|_| {
                    pixel_color = pixel_color + ray_color(camera.gen_ray(x, y), world, 0);
                });

                buffer[screen_pos] + (pixel_color / SAMPLES as f32)
            },
        )
        .collect()
}
