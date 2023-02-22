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
use std::{sync::Arc, time::Duration};

use camera::Camera;
use hittable::{ABox, Bvh, Hittable, Sphere};
use material::Material;
use minifb::{Key, Window, WindowOptions};
use ultraviolet::Vec3;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

const ASPECT_RATIO: f32 = 3.0 / 2.0;

const WIDTH: usize = 800;
const HEIGHT: usize = (WIDTH as f32 / ASPECT_RATIO) as usize;

const SAMPLES: u32 = 128;

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

    // let camera = &mut Camera::new(
    //     Vec3::new(4.0, 2.0, 1.5),
    //     Vec3::new(2.0, 1.2, 0.0),
    //     Vec3::new(0.0, 1.0, 0.0),
    //     60.0,
    //     ASPECT_RATIO,
    //     0.01,
    //     2.0,
    // );

    let camera = &mut Camera::new(
        Vec3::new(0.7, -0.2, -4.5),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        ASPECT_RATIO,
        0.2,
        5.0,
    );

    // World setup
    let world = Arc::new(box_scene());
    let mut pass: u32 = 0;
    let mut total_times: Duration = Default::default();

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
            let now = std::time::Instant::now();
            buffer = render::render(
                WIDTH,
                HEIGHT,
                *camera,
                world.clone(),
                buffer.as_slice(),
                SAMPLES,
                MAX_BOUNCE,
            );
            let elapsed_time = now.elapsed();
            total_times += elapsed_time;
            println!("Frame took {} seconds.", elapsed_time.as_secs_f32());
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
    println!(
        "Average frame time {} seconds.",
        total_times.as_secs_f32() / pass as f32
    );
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

    let glass = Material::dielectric((0.6, 0.1, 0.2), 1.51);
    let _steel = Material::metal((0.65, 0.65, 0.65), 0.2, 1.5);
    let light = Material::lambertian((12.0, 12.0, 12.0));
    let diffuse = Material::lambertian((0.5, 0.5, 0.5));
    let _diffuse_black = Material::lambertian((0.01, 0.01, 0.01));
    let _diffuse_red = Material::lambertian((0.9, 0.1, 0.1));
    let diffuse_green = Material::lambertian((0.1, 0.9, 0.1));
    let _diffuse_blue = Material::lambertian((0.1, 0.1, 0.9));

    world.push(Arc::new(Sphere::new((1.5, -1.0, 2.5), 1.0, diffuse_green)));
    world.push(Arc::new(Sphere::new((-1.5, -1.0, 0.5), 1.0, glass)));
    // world.push(Arc::new( Sphere::new((-1.5, -1.0, 0.5), -0.8, glass)));
    world.push(Arc::new(ABox::new(
        (0.0, 3.0, 2.0),
        (1.5, 0.01, 1.5),
        light,
    )));

    world.push(Arc::new(ABox::new(
        (0.0, 0.5, -0.4),
        (-6.0, -5.0, -8.4),
        diffuse,
    )));

    // world.push(Arc::new(Sphere::new((0.0, 503.0, 0.0), 500.0, diffuse)));
    // world.push(Arc::new(Sphere::new(
    //     (0.0, -502.0, 0.0),
    //     500.0,
    //     diffuse_green,
    // )));

    // world.push(Arc::new(Sphere::new(
    //     (503.0, 0.0, 0.0),
    //     500.0,
    //     diffuse_blue,
    // )));
    // world.push(Arc::new(Sphere::new(
    //     (-503.0, 0.0, 0.0),
    //     500.0,
    //     diffuse_red,
    // )));

    // world.push(Arc::new(Sphere::new((0.0, 0.0, 505.0), 500.0, diffuse)));
    // world.push(Arc::new(Sphere::new(
    //     (0.0, 0.0, -505.0),
    //     500.0,
    //     diffuse_black,
    // )));
    Bvh::new(&mut world)
}

#[ignore]
fn random_scene() -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    let ground: Material = Material::lambertian((0.5, 0.5, 0.5));
    world.push(Arc::new(ABox::new(
        (0.0, -1.0, 0.0),
        (100.0, 2.0, 100.0),
        ground,
    )));
    for a in -3..3 {
        for b in -3..3 {
            let choose_mat = fastrand::f32();
            let center = (
                a as f32 + 0.9 * fastrand::f32(),
                0.2 + choose_mat,
                b as f32 + 0.9 * fastrand::f32(),
            );

            if (Vec3::new(center.0, center.1, center.2) - Vec3::new(4.0, 0.2, 0.0)).mag() > 0.9 {
                let albedo = (
                    fastrand::f32() * fastrand::f32(),
                    fastrand::f32() * fastrand::f32(),
                    fastrand::f32() * fastrand::f32(),
                );
                if choose_mat < 0.6 {
                    // diffuse
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::lambertian(albedo),
                    )));
                } else if choose_mat < 0.9 {
                    // metal
                    let fuzz = 0.5 * fastrand::f32();
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::metal((albedo.0, albedo.1, albedo.2), fuzz, 5.0),
                    )));
                } else {
                    // glass
                    world.push(Arc::new(Sphere::new(
                        center,
                        0.2,
                        Material::dielectric(
                            (fastrand::f32(), fastrand::f32(), fastrand::f32()),
                            1.52,
                        ),
                    )));
                }
            }
        }
    }

    let glass = Material::dielectric((0.6, 0.1, 0.25), 1.52);
    let _steel = Material::metal((0.7, 0.7, 0.7), 0.01, 2.1);
    let _diffuse = Material::lambertian((0.2, 0.7, 0.8));

    world.push(Arc::new(ABox::new(
        (-0.5, 1.0, 0.0),
        (-1.8, -1.8, -1.8),
        glass,
    )));
    world.push(Arc::new(ABox::new(
        (-0.5, 1.0, 0.0),
        (2.0, 2.0, 2.0),
        glass,
    )));
    world.push(Arc::new(ABox::new((2.0, 1.2, 0.0), (2.0, 2.0, 0.2), glass)));
    // world.push(Arc::new(Sphere::new(
    //     (-4.0, 1.0, 0.0),
    //     1.0,
    //     diffuse,
    // )));
    // world.push(Arc::new(Sphere::new((4.0, 1.0, 0.0), 1.0, steel)));

    Bvh::new(&mut world)
}
