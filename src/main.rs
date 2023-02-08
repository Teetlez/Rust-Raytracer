mod camera;
mod hittable;
mod ray;
mod vec3;

extern crate minifb;

use camera::Camera;
use hittable::{Sphere, World};
use minifb::{clamp, Key, Window, WindowOptions};

use ray::Ray;
use std::f32::INFINITY;
use vec3::Vec3;

const PI: f32 = 3.1415926535897932385;

const ASPECT_RATIO: f32 = 16.0 / 9.0;

const WIDTH: usize = 800;
const HEIGHT: usize = (WIDTH as f32 / ASPECT_RATIO) as usize;

const SAMPLES: usize = 128;

const MAX_BOUNCE: usize = 4;

const MAX_PASSES: u32 = 8;

#[inline]
fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * PI / 180.0
}

#[inline]
fn coord_to_index(x: usize, y: usize) -> usize {
    ((HEIGHT - 1 - y) * WIDTH) + x
}

#[inline]
fn to_rgb(color: Vec3) -> u32 {
    255 << 24
        | ((clamp(0.0, color.x.sqrt(), 0.99) * 255.4) as u32) << 16
        | ((clamp(0.0, color.y.sqrt(), 0.99) * 255.4) as u32) << 8
        | ((clamp(0.0, color.z.sqrt(), 0.99) * 255.4) as u32)
}

#[inline]
fn mix_colors(color1: u32, color2: u32, weight1: f32) -> u32 {
    let weight2 = 1.0 - weight1;

    255 << 24
        | (((weight2 * ((color1 >> 16) & 0xff) as f32 + weight1 * ((color2 >> 16) & 0xff) as f32)
            as u32)
            << 16)
        | (((weight2 * ((color1 >> 8) & 0xff) as f32 + weight1 * ((color2 >> 8) & 0xff) as f32)
            as u32)
            << 8)
        | ((weight2 * (color1 & 0xff) as f32 + weight1 * (color2 & 0xff) as f32) as u32)
}

#[inline]
fn ray_color(ray: Ray, world: &World, depth: usize) -> Vec3 {
    if depth >= MAX_BOUNCE {
        return Vec3::new(0.0, 0.0, 0.0);
    }
    if let Some(hit) = world.hit(&ray, 0.0001, INFINITY) {
        let target: Vec3 = hit.point + hit.normal + Vec3::random_unit_vector();
        0.5 * ray_color(Ray::new(hit.point, target - hit.point), world, depth + 1)
    } else {
        let t = 0.5 * (ray.dir.y + 1.0);
        (1.0 - t) * Vec3::new(1.0, 1.0, 1.0) + t * Vec3::new(0.5, 0.7, 1.0)
    }
}

fn main() {
    // Window setup
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // Make camera
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);

    // World setup
    let world = World::new(vec![
        Sphere::new(Vec3::new(0.0, -0.05, -1.0), 0.5),
        Sphere::new(Vec3::new(1.0, 0.05, -1.0), 0.5),
        Sphere::new(Vec3::new(-1.5, 0.0, -1.5), 0.5),
        Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0),
    ]);
    let mut pass: u32 = 0;

    // Render loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if pass <= MAX_PASSES {
            pass += 1;
            fastrand::seed(fastrand::get_seed() + pass as u64);
            for y in 0..HEIGHT {
                // if y % 100 == 0 {
                //     let percent = (y as f32 / HEIGHT as f32) * 100.0;
                //     println!("{percent}% done");
                // }
                for x in 0..WIDTH {
                    let mut pixel_color = Vec3::new(0.0, 0.0, 0.0);
                    (0..SAMPLES).for_each(|_| {
                        pixel_color = pixel_color + ray_color(camera.gen_ray(x, y), &world, 0);
                    });
                    buffer[coord_to_index(x, y)] = mix_colors(
                        buffer[coord_to_index(x, y)],
                        to_rgb(pixel_color / SAMPLES as f32),
                        1.0 / pass as f32,
                    );
                }
            }
            // println!("done!");
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
        } else {
            window.update();
        }
    }
}
