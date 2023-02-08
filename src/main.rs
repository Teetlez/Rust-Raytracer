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

use vec3::Vec3;

const ASPECT_RATIO: f32 = 3.0 / 2.0;

const WIDTH: usize = 800;
const HEIGHT: usize = (WIDTH as f32 / ASPECT_RATIO) as usize;

const SAMPLES: usize = 128;

const MAX_BOUNCE: usize = 8;

const MAX_PASSES: u32 = 1;

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
        Vec3::new(13.0, 2.0, 3.0),
        Vec3::zero(),
        Vec3::new(0.0, 1.0, 0.0),
        20.0,
        ASPECT_RATIO,
        0.1,
        10.0,
    );

    // World setup
    let world = random_scene();
    let mut pass: u32 = 0;

    // Render loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if pass <= MAX_PASSES {
            pass += 1;
            fastrand::seed(fastrand::get_seed() + pass as u64);

            // println!("done!");
            window
                .update_with_buffer(
                    render::render(WIDTH, HEIGHT, camera, &world).as_slice(),
                    WIDTH,
                    HEIGHT,
                )
                .unwrap();
            println!("finished pass {pass}");
        } else {
            window.update();
        }
    }
}

fn random_scene() -> World {
    let mut world: World = World::new(vec![]);
    let ground: Material = Material::lambertian(0.5 * Vec3::one());
    world
        .hittables
        .push(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, ground));
    for a in -11..11 {
        for b in -11..11 {
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
                    world
                        .hittables
                        .push(Sphere::new(center, 0.2, Material::lambertian(albedo)));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo = (0.5 * Vec3::random()) + 0.5 * Vec3::one();
                    let fuzz = 0.5 * fastrand::f32();
                    world
                        .hittables
                        .push(Sphere::new(center, 0.2, Material::metal(albedo, fuzz)));
                } else {
                    // glass
                    world.hittables.push(Sphere::new(
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

    world
        .hittables
        .push(Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0, steel));

    world
        .hittables
        .push(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0, diffuse));

    world
        .hittables
        .push(Sphere::new(Vec3::new(4.0, 1.0, 0.0), 1.0, glass));

    world
}
