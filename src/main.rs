#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod camera;
mod hittable;
mod material;
mod random;
mod ray;
mod render;

extern crate minifb;
extern crate ultraviolet;
use std::sync::Arc;

use camera::Camera;
use hittable::{Bvh, Hittable, Sphere};
use material::Material;
use minifb::{Key, Window, WindowOptions};
use random::random_vec;
use ultraviolet::Vec3;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

const ASPECT_RATIO: f32 = 3.0 / 2.0;

const WIDTH: usize = 400;
const HEIGHT: usize = (WIDTH as f32 / ASPECT_RATIO) as usize;

const SAMPLES: u32 = 32;

const MAX_BOUNCE: u32 = 8;

const MAX_PASSES: u32 = 64;

fn main() {
    // Window setup
    let mut buffer: Vec<Vec3> = vec![Vec3::zero(); WIDTH * HEIGHT];

    let mut window = Window::new("Rust Pathtracer", WIDTH, HEIGHT, WindowOptions::default())
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // Make camera

    let camera = &mut Camera::new(
        Vec3::new(13.0, 2.0, 3.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        20.0,
        ASPECT_RATIO,
        0.1,
        10.0,
    );

    // let camera = &mut Camera::new(
    //     Vec3::new(0.7, -0.2, -4.5),
    //     Vec3::new(0.0, 0.0, 0.0),
    //     Vec3::new(0.0, 1.0, 0.0),
    //     60.0,
    //     ASPECT_RATIO,
    //     0.2,
    //     5.0,
    // );

    // World setup
    let world = Arc::new(random_scene());
    let mut pass: u32 = 0;

    println!("press Enter to start render");
    // Render loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        while pass == 0 && window.is_open() {
            if window.get_mouse_down(minifb::MouseButton::Left) {
                window.set_cursor_visibility(false);
                camera.update(
                    window.get_keys(),
                    window.get_mouse_pos(minifb::MouseMode::Pass),
                    window.get_scroll_wheel().get_or_insert((0.0, 0.0)).1,
                );
            } else {
                window.set_cursor_visibility(true);
                window.set_cursor_style(minifb::CursorStyle::Arrow);
            }
            if window.is_key_pressed(Key::Enter, minifb::KeyRepeat::Yes) {
                println!("Rendering with {MAX_PASSES} passes");
                buffer = vec![Vec3::zero(); WIDTH * HEIGHT];
                break;
            }
            buffer = render::preview(WIDTH, HEIGHT, *camera, world.clone());
            window
                .update_with_buffer(
                    buffer
                        .par_iter()
                        .map(to_rgb)
                        .collect::<Vec<u32>>()
                        .as_slice(),
                    WIDTH,
                    HEIGHT,
                )
                .unwrap();
            pass = 0;
        }
        if pass <= MAX_PASSES && window.is_open() {
            pass += 1;
            println!("rendering...");
            buffer = render::render(
                WIDTH,
                HEIGHT,
                *camera,
                world.clone(),
                buffer.as_slice(),
                SAMPLES,
                MAX_BOUNCE,
            );
            println!("writing to buffer");
            window
                .update_with_buffer(
                    buffer
                        .par_iter()
                        .map(|color| to_rgb(&(*color / pass as f32)))
                        .collect::<Vec<u32>>()
                        .as_slice(),
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

#[inline]
fn to_rgb(color: &Vec3) -> u32 {
    255 << 24
        | ((minifb::clamp(0.0, color.x.powf(1.0 / 2.2), 0.999) * 255.4) as u32) << 16
        | ((minifb::clamp(0.0, color.y.powf(1.0 / 2.2), 0.999) * 255.4) as u32) << 8
        | ((minifb::clamp(0.0, color.z.powf(1.0 / 2.2), 0.999) * 255.4) as u32)
}

fn box_scene() -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];

    let glass = Material::dielectric(Vec3::new(0.6, 0.1, 0.2), 1.51);
    let steel = Material::metal(0.65 * Vec3::one(), 0.2, 1.5);
    let light = Material::lambertian(16.0 * Vec3::one());
    let diffuse = Material::lambertian(Vec3::new(0.7, 0.7, 0.7));
    let diffuse_black = Material::lambertian(Vec3::new(0.01, 0.01, 0.01));
    let diffuse_red = Material::lambertian(Vec3::new(0.9, 0.1, 0.1));
    let diffuse_green = Material::lambertian(Vec3::new(0.1, 0.9, 0.1));
    let diffuse_blue = Material::lambertian(Vec3::new(0.1, 0.1, 0.9));

    world.push(Arc::new(Sphere::new(Vec3::new(1.5, -1.0, 2.5), 1.0, steel)));
    world.push(Arc::new(Sphere::new(
        Vec3::new(-1.5, -1.0, 0.5),
        1.0,
        glass,
    )));
    // world.push(Arc::new( Sphere::new(Vec3::new(-1.5, -1.0, 0.5), -0.8, glass)));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 102.999, 2.0),
        100.0,
        light,
    )));

    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 503.0, 0.0),
        500.0,
        diffuse,
    )));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, -502.0, 0.0),
        500.0,
        diffuse_green,
    )));

    world.push(Arc::new(Sphere::new(
        Vec3::new(503.0, 0.0, 0.0),
        500.0,
        diffuse_blue,
    )));
    world.push(Arc::new(Sphere::new(
        Vec3::new(-503.0, 0.0, 0.0),
        500.0,
        diffuse_red,
    )));

    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 0.0, 505.0),
        500.0,
        diffuse,
    )));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 0.0, -505.0),
        500.0,
        diffuse_black,
    )));
    Bvh::new(&mut world)
}

#[ignore]
fn random_scene() -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    let ground: Material = Material::lambertian(0.5 * Vec3::one());
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, -1000.0, 0.0),
        1000.0,
        ground,
    )));
    for a in -8..8 {
        for b in -6..6 {
            let choose_mat = fastrand::f32();
            let center = Vec3::new(
                a as f32 + 0.9 * fastrand::f32(),
                0.2,
                b as f32 + 0.9 * fastrand::f32(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).mag() > 0.9 {
                if choose_mat < 0.6 {
                    // diffuse
                    let albedo = random_vec() * random_vec();
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::lambertian(albedo),
                    )));
                } else if choose_mat < 0.9 {
                    // metal
                    let albedo = (0.5 * random_vec()) + 0.5 * Vec3::one();
                    let fuzz = 0.5 * fastrand::f32();
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::metal(albedo, fuzz, 5.0),
                    )));
                } else {
                    // glass
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::dielectric(
                            Vec3::new(fastrand::f32(), fastrand::f32(), fastrand::f32()),
                            1.52,
                        ),
                    )));
                }
            }
        }
    }

    let glass = Material::dielectric(Vec3::new(0.6, 0.1, 0.25), 1.52);
    let steel = Material::metal(0.7 * Vec3::one(), 0.2, 2.1);
    let diffuse = Material::lambertian(Vec3::new(0.2, 0.7, 0.8));

    world.push(Arc::new(Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0, glass)));
    world.push(Arc::new(Sphere::new(
        Vec3::new(-4.0, 1.0, 0.0),
        1.0,
        diffuse,
    )));
    world.push(Arc::new(Sphere::new(Vec3::new(4.0, 1.0, 0.0), 1.0, steel)));

    Bvh::new(&mut world)
}
