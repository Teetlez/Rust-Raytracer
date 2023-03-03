#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod camera;
mod filter;
mod hittable;
mod material;
mod random;
mod ray;
mod render;

extern crate clap;
extern crate minifb;
extern crate ultraviolet;
use std::{fs::File, io::BufReader, sync::Arc, time::Duration};

use camera::Camera;
use clap::Parser;
use hittable::{ABox, Bvh, Hittable, Sphere};
use material::Material;
use minifb::{Key, Window, WindowOptions};
use ultraviolet::Vec3;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{filter::bilateral_filter, render::Renderer};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    // Number of samples per pixel
    #[arg(short, long, default_value_t = 128)]
    samples: u32,

    // Number of frames to cumulate
    #[arg(short, long, default_value_t = 128)]
    passes: u32,

    // Max number of times a ray can bounce
    #[arg(short, long, default_value_t = 8)]
    bounces: u32,

    // Pixel width of frame
    #[arg(short, long, default_value_t = 640)]
    width: usize,

    // Pixel hight of frame
    #[arg(short, long, default_value_t = 480)]
    height: usize,

    // Gamma level
    #[arg(short, long, default_value_t = 2.2)]
    gamma: f32,

    // Max light brightness
    #[arg(short, long, default_value_t = -1.0)]
    clamp_light: f32,

    // apply bilateral filter after render
    #[arg(short, long, default_value_t = false)]
    filter: bool,
}

fn main() {
    let args = Args::parse();

    // Window setup
    let mut buffer: Vec<Vec3> = vec![Vec3::zero(); args.width * args.height];

    let f = File::open(r"C:\Git_Projects\Rust-Raytracer\Scene\HDR\studio_small_08_2k.hdr")
        .expect("Failed to open specified file");
    let f = BufReader::new(f);
    let image = Arc::new(radiant::load(f).expect("Failed to load image data"));

    let mut window = Window::new(
        "Rust Pathtracer",
        args.width,
        args.height,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // Make camera

    // let camera = &mut Camera::new(
    //     Vec3::new(13.0, 2.0, 3.0),
    //     Vec3::new(0.0, 0.0, 0.0),
    //     Vec3::new(0.0, 1.0, 0.0),
    //     20.0,
    //     args.width as f32 / args.height as f32,
    //     0.15,
    //     11.0,
    // );
    let camera = &mut Camera::new(
        Vec3::new(0.7, -0.2, -4.5),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        args.width as f32 / args.height as f32,
        0.1,
        5.0,
    );

    // World setup
    // let world = Arc::new(_random_scene(false, true, true, true, true));
    let world = Arc::new(_box_scene());
    let mut pass: u32 = 0;
    let mut total_times: Duration = Default::default();

    let renderer = Renderer {
        width: args.width,
        height: args.height,
        camera: *camera,
        world,
        sample_rate: args.samples,
        max_bounce: args.bounces,
        hdr: image,
    };

    let gamma = args.gamma.recip();

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
                println!(
                    "Rendering with {0} passes with {1} rays per pixel",
                    args.passes, renderer.sample_rate
                );
                buffer = vec![Vec3::zero(); renderer.height * renderer.width];
                break;
            }
            buffer = renderer.preview();
            window
                .update_with_buffer(
                    buffer
                        .par_iter()
                        .map(|color| render::to_rgb(color, gamma))
                        .collect::<Vec<u32>>()
                        .as_slice(),
                    renderer.width,
                    renderer.height,
                )
                .unwrap();
            pass = 0;
        }
        if pass < args.passes && window.is_open() {
            pass += 1;
            total_times += {
                println!("rendering...");
                let now = std::time::Instant::now();
                buffer = renderer.render(buffer.as_slice());
                let elapsed_time = now.elapsed();
                println!("Frame took {} seconds.", elapsed_time.as_secs_f32());
                window
                    .update_with_buffer(
                        buffer
                            .par_iter()
                            .map(|color| render::to_rgb(&(*color / pass as f32), gamma))
                            .collect::<Vec<u32>>()
                            .as_slice(),
                        args.width,
                        args.height,
                    )
                    .unwrap();
                println!("finished pass {pass}");
                elapsed_time
            };
        } else if window.is_open() {
            break;
        }
    }
    println!(
        "Average frame time {} seconds.",
        total_times.as_secs_f32() / pass as f32
    );
    // Normalize samples
    buffer = buffer.iter().map(|color| (*color / pass as f32)).collect();

    if args.filter {
        // Apply Bilateral filter
        (0..args.width * args.height).for_each(|index| {
            buffer[index] = bilateral_filter(
                &buffer[index],
                index,
                &buffer,
                args.width as u32,
                args.height as u32,
                3,
                0.05,
                1.0,
            )
        });
    }
    // Solidify image buffer
    let final_img = buffer
        .iter()
        .map(|color| render::to_rgb(color, gamma))
        .collect::<Vec<u32>>();

    // Loop until closed
    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(final_img.as_slice(), args.width, args.height)
            .unwrap();
    }
}

#[allow(dead_code)]
fn _box_scene() -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];

    let glass = Material::dielectric((0.6, 0.1, 0.25), 1.52);
    let steel = Material::metal((0.65, 0.65, 0.65), 0.2, 1.5);
    let light = Material::lambertian((12.0, 12.0, 12.0));
    let diffuse = Material::lambertian((0.5, 0.5, 0.5));
    let diffuse_black = Material::lambertian((0.01, 0.01, 0.01));
    let diffuse_red = Material::lambertian((0.9, 0.1, 0.1));
    let diffuse_green = Material::lambertian((0.1, 0.9, 0.1));
    let diffuse_blue = Material::lambertian((0.1, 0.1, 0.9));

    world.push(Arc::new(Sphere::new((1.5, -1.0, 2.5), 1.0, steel)));
    world.push(Arc::new(Sphere::new((-1.5, -1.0, 0.5), 1.0, glass)));
    // world.push(Arc::new( Sphere::new((-1.5, -1.0, 0.5), -0.8, glass)));
    world.push(Arc::new(ABox::new(
        (0.0, 3.0, 1.5),
        (2.5, 0.01, 2.5),
        light,
    )));

    // world.push(Arc::new(ABox::new(
    //     (0.0, 0.5, -0.4),
    //     (-6.0, -5.0, -8.4),
    //     diffuse,
    // )));

    world.push(Arc::new(Sphere::new((0.0, 503.0, 0.0), 500.0, diffuse)));
    world.push(Arc::new(Sphere::new(
        (0.0, -502.0, 0.0),
        500.0,
        diffuse_green,
    )));

    world.push(Arc::new(Sphere::new(
        (503.0, 0.0, 0.0),
        500.0,
        diffuse_blue,
    )));
    world.push(Arc::new(Sphere::new(
        (-503.0, 0.0, 0.0),
        500.0,
        diffuse_red,
    )));

    world.push(Arc::new(Sphere::new((0.0, 0.0, 505.0), 500.0, diffuse)));
    world.push(Arc::new(Sphere::new(
        (0.0, 0.0, -505.0),
        500.0,
        diffuse_black,
    )));
    Bvh::new(&mut world)
}

#[allow(dead_code)]
fn _random_scene(lights: bool, diffuse: bool, glossy: bool, metal: bool, glass: bool) -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    let ground: Material = Material::lambertian((0.5, 0.5, 0.5));
    world.push(Arc::new(ABox::new(
        (-2.0, -0.5, -2.0),
        (50.0, 1.0, 50.0),
        ground,
    )));
    if lights || diffuse || glossy || metal || glass {
        for a in -11..11 {
            for b in -10..7 {
                let choose_mat = fastrand::f32();
                let center = (
                    a as f32 + 0.9 * fastrand::f32(),
                    0.2,
                    b as f32 + 0.9 * fastrand::f32(),
                );

                if (Vec3::new(center.0, center.1, center.2) - Vec3::new(4.0, 0.2, 0.0)).mag() > 0.9
                {
                    let albedo = (
                        fastrand::f32() * fastrand::f32(),
                        fastrand::f32() * fastrand::f32(),
                        fastrand::f32() * fastrand::f32(),
                    );
                    if glossy && choose_mat < 0.3 {
                        // glossy
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::glossy(
                                albedo,
                                fastrand::f32() * 0.5,
                                (fastrand::f32() * 0.5) + 1.0,
                            ),
                        )));
                    } else if diffuse && choose_mat < 0.6 {
                        // diffuse
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::lambertian(albedo),
                        )));
                    } else if metal && choose_mat < 0.8 {
                        // metal
                        let fuzz = 0.5 * fastrand::f32();
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::metal((albedo.0, albedo.1, albedo.2), fuzz, 5.0),
                        )));
                    } else if lights && choose_mat < 0.9 {
                        // lights
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::lambertian((
                                fastrand::f32() * 6.0,
                                fastrand::f32() * 6.0,
                                fastrand::f32() * 6.0,
                            )),
                        )));
                    } else if glass {
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
    }

    let glass = Material::dielectric((0.1, 0.1, 0.1), 1.52);
    let gloss = Material::glossy((0.2, 0.1, 0.05), 0.2, 0.28);
    let steel = Material::metal((0.7, 0.6, 0.5), 0.01, 1.1);
    // let diffuse = Material::lambertian((0.4, 0.2, 0.1));

    world.push(Arc::new(Sphere::new((4.0, 1.0, 0.0), 1.0, steel)));
    world.push(Arc::new(Sphere::new((0.0, 1.0, 0.0), 1.0, glass)));
    world.push(Arc::new(Sphere::new((-4.0, 1.0, 0.0), 1.0, gloss)));
    // world.push(Arc::new(Sphere::new((-4.5, 1.0, 0.0), 1.0, diffuse)));

    Bvh::new(&mut world)
}
