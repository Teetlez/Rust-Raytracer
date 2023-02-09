#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod camera;
mod hittable;
mod material;
mod ray;
mod render;
mod vec3;

extern crate minifb;

use camera::Camera;
use hittable::{Sphere, World};
use material::Material;
use minifb::{Key, Window, WindowOptions};

use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use vec3::Vec3;

const ASPECT_RATIO: f32 = 3.0 / 2.0;

const WIDTH: usize = 800;
const HEIGHT: usize = (WIDTH as f32 / ASPECT_RATIO) as usize;

const SAMPLES: usize = 128;

const MAX_BOUNCE: usize = 8;

const MAX_PASSES: u32 = 32;

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
    let camera = Camera::new(
        Vec3::new(0.0, 0.5, -4.0),
        Vec3::zero(),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        ASPECT_RATIO,
        0.1,
        5.0,
    );

    // World setup
    let world = box_scene();
    let mut pass: u32 = 0;

    // Render loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if pass <= MAX_PASSES {
            pass += 1;

            // println!("done!");
            buffer = buffer
                .into_par_iter()
                .zip_eq(render::render(WIDTH, HEIGHT, camera, &world).into_par_iter())
                .map(|(c1, c2)| mix_colors(c1, c2, 1.0 / pass as f32))
                .collect();
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
            println!("finished pass {pass}");
        } else {
            window.update();
        }
    }
}

fn box_scene() -> World {
    let mut world: World = World::new(vec![]);

    let glass = Material::dielectric(1.46);
    let steel = Material::metal(0.7 * Vec3::one(), 0.2);
    let light = Material::lambertian(16.0 * Vec3::one());
    let diffuse = Material::lambertian(Vec3::new(0.5, 0.5, 0.5));
    let diffuse_red = Material::lambertian(Vec3::new(0.9, 0.1, 0.1));
    let diffuse_green = Material::lambertian(Vec3::new(0.1, 0.9, 0.1));
    let diffuse_blue = Material::lambertian(Vec3::new(0.1, 0.1, 0.9));

    world.add(Sphere::new(Vec3::new(1.5, -1.0, 2.0), 1.0, steel));
    world.add(Sphere::new(Vec3::new(-1.5, -1.0, 0.5), 1.0, glass));
    world.add(Sphere::new(Vec3::new(0.0, 102.999, 2.0), 100.0, light));

    world.add(Sphere::new(Vec3::new(0.0, 503.0, 0.0), 500.0, diffuse));
    world.add(Sphere::new(
        Vec3::new(0.0, -502.0, 0.0),
        500.0,
        diffuse_blue,
    ));

    world.add(Sphere::new(Vec3::new(503.5, 0.0, 0.0), 500.0, diffuse_red));
    world.add(Sphere::new(
        Vec3::new(-503.5, 0.0, 0.0),
        500.0,
        diffuse_green,
    ));

    world.add(Sphere::new(Vec3::new(0.0, 0.0, 505.0), 500.0, diffuse));
    world
}

#[ignore]
fn random_scene() -> World {
    let mut world: World = World::new(vec![]);
    let ground: Material = Material::lambertian(0.5 * Vec3::one());
    world.add(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, ground));
    for a in -8..8 {
        for b in -8..8 {
            let choose_mat = fastrand::f32();
            let center = Vec3::new(
                a as f32 + 0.9 * fastrand::f32(),
                0.2,
                b as f32 + 0.9 * fastrand::f32(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.8 {
                    // diffuse
                    let albedo = Vec3::random() * Vec3::random();
                    world.add(Sphere::new(center, 0.2, Material::lambertian(albedo)));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo = (0.5 * Vec3::random()) + 0.5 * Vec3::one();
                    let fuzz = 0.5 * fastrand::f32();
                    world.add(Sphere::new(center, 0.2, Material::metal(albedo, fuzz)));
                } else {
                    // glass
                    world.add(Sphere::new(
                        center,
                        0.2,
                        Material::dielectric(fastrand::f32() + 1.2),
                    ));
                }
            }
        }
    }

    let glass = Material::dielectric(1.46);
    let steel = Material::metal(0.7 * Vec3::one(), 0.2);
    let diffuse = Material::lambertian(Vec3::new(0.2, 0.7, 0.8));

    world.add(Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0, steel));
    world.add(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0, diffuse));
    world.add(Sphere::new(Vec3::new(4.0, 1.0, 0.0), 1.0, glass));

    world
}
