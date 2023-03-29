#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
use camera::Camera;
use clap::Parser;
use minifb::{Key, Window, WindowOptions};

use std::{
    f32::INFINITY,
    fs::File,
    io::BufReader,
    path::Path,
    sync::Arc,
    time::{self, Duration},
};
use ultraviolet::Vec3;

use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{filter::bilateral_filter, render::Renderer};

pub mod camera;
pub mod filter;
pub mod io;
pub mod material;
pub mod random;
pub mod ray;
pub mod render;
pub mod tracer;

extern crate clap;
extern crate minifb;
extern crate serde;
extern crate ultraviolet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Scene file to use
    scene: Option<String>,

    /// Number of samples per pixel
    #[arg(short, long, default_value_t = 128)]
    samples: u32,

    /// Number of frames to cumulate
    #[arg(short, long, default_value_t = 64)]
    passes: u32,

    /// Max number of times a ray can bounce
    #[arg(short, long, default_value_t = 8)]
    bounces: u32,

    /// Pixel width of frame
    #[arg(long, default_value_t = 600)]
    width: usize,

    /// Pixel hight of frame
    #[arg(long, default_value_t = 400)]
    height: usize,

    /// Gamma level
    #[arg(short, long, default_value_t = 2.2)]
    gamma: f32,

    /// Max light brightness
    #[arg(short, long, default_value_t = INFINITY)]
    light_clamp: f32,

    /// apply bilateral filter after render to reduce noise
    #[arg(short, long, default_value_t = false)]
    filter: bool,
}

fn main() {
    let args = Args::parse();

    // Window setup
    let mut buffer: Vec<Vec3> = vec![Vec3::zero(); args.width * args.height];

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

    let mut renderer = if let Some(path) = &args.scene {
        io::load_scene(Path::new(path), &args).unwrap()
    } else {
        // Load HDR
        let image = if let Ok(f) = File::open(r".\scene\HDR\lythwood_room.hdr") {
            let reader = BufReader::new(f);
            Arc::new(radiant::load(reader).ok())
        } else {
            Arc::new(None)
        };

        // Make camera
        let camera = &mut Camera::new(
            Vec3::new(13.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::unit_y(),
            20.0,
            args.width as f32 / args.height as f32,
            0.1,
            10.0,
        );

        // World setup
        let world = Arc::new(io::random_scene(true, true, true, true, true));
        Renderer {
            width: args.width,
            height: args.height,
            camera: *camera,
            world,
            sample_rate: args.samples,
            max_bounce: args.bounces,
            hdr: image,
            light_clamp: INFINITY,
        }
    };

    let mut pass: u32 = 0;
    let mut total_times: Duration = Default::default();

    let gamma = args.gamma.recip();

    println!("press Enter to start render");
    // Render loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        while pass == 0 && window.is_open() {
            if window.get_mouse_down(minifb::MouseButton::Left) {
                window.set_cursor_visibility(false);
                renderer.camera.update(
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
        buffer = (0..renderer.width * renderer.height)
            .into_par_iter()
            .map(|index| {
                bilateral_filter(
                    &buffer[index],
                    index,
                    buffer.as_slice(),
                    (args.width as u32, args.height as u32),
                    3,
                    0.05,
                    1.0,
                )
            })
            .collect();
    }
    // Solidify image buffer
    let final_img = buffer
        .par_iter()
        .map(|color| render::to_rgb(color, gamma))
        .collect::<Vec<u32>>();

    println!("Press ENTER to save as png");
    // Loop until closed
    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(final_img.as_slice(), renderer.width, renderer.height)
            .unwrap();
        if window.is_key_down(Key::Enter) {
            io::save_colors_as_image(
                final_img.as_slice(),
                renderer.width as u32,
                renderer.height as u32,
                &format!(
                    "output/{}.png",
                    time::SystemTime::now()
                        .duration_since(time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ),
            )
            .ok();
            break;
        }
    }
}
