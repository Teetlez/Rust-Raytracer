// Originally written in 2023 by Arman Uguray <arman.uguray@gmail.com>
// SPDX-License-Identifier: CC-BY-4.0

use anyhow::{Context, Result};
use clap::Parser;
use std::{
    collections::HashSet,
    fs::File,
    io::BufReader,
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use ultraviolet::Vec3;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseScrollDelta, RawKeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[allow(dead_code)]
mod camera;
mod gpu_camera;
mod gpu_render;
mod gpu_scene;
mod io;
#[allow(dead_code)]
mod material;
#[allow(dead_code)]
mod random;
#[allow(dead_code)]
mod ray;
#[allow(dead_code)]
mod render;
#[allow(dead_code)]
mod tracer;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    scene: Option<String>,
    #[arg(short, long, default_value_t = 128)]
    pub samples: u32,
    #[arg(short, long, default_value_t = 64)]
    pub passes: u32,
    #[arg(short, long, default_value_t = 8)]
    pub bounces: u32,
    #[arg(long, default_value_t = 600)]
    pub width: usize,
    #[arg(long, default_value_t = 400)]
    pub height: usize,
    #[arg(short, long, default_value_t = 2.2)]
    pub gamma: f32,
    #[arg(short, long, default_value_t = f32::INFINITY)]
    pub light_clamp: f32,
    #[arg(short, long, default_value_t = false)]
    pub filter: bool,
}

#[pollster::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cpu_renderer = if let Some(path) = &args.scene {
        io::load_scene(Path::new(path), &args)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
    } else {
        make_default_setup(&args)
    };
    let gpu_scene = cpu_renderer.world.to_gpu_scene();
    let (eye, look_direction, fov, aspect_ratio, aperture, focus_dist) =
        cpu_renderer.camera.gpu_parameters();
    let width = args.width as u32;
    let height = args.height as u32;

    let event_loop = EventLoop::new()?;
    let window_size = winit::dpi::PhysicalSize::new(width, height);
    let window_attributes = WindowAttributes::default()
        .with_inner_size(window_size)
        .with_resizable(false)
        .with_title("GPU Path Tracer");
    let window = event_loop.create_window(window_attributes)?;
    let (device, queue, surface) = connect_to_gpu(&window).await?;
    let format = surface
        .get_configuration()
        .context("surface was not configured")?
        .format;
    let mut renderer = gpu_render::PathTracer::new(
        device,
        queue,
        width,
        height,
        format,
        &gpu_scene,
        cpu_renderer.hdr.as_ref().as_ref(),
        args.bounces,
        args.samples,
        args.passes,
        args.gamma,
        args.light_clamp,
    );
    let mut camera = gpu_camera::Camera::new(
        eye,
        look_direction,
        Vec3::unit_y(),
        fov,
        aspect_ratio,
        aperture,
        focus_dist,
    );
    camera.update_uniforms();
    let mut filter_enabled = args.filter;
    renderer.set_filter_enabled(filter_enabled);
    let mut setup_mode = true;
    renderer.set_quality(1, 1);
    println!("setup mode: 1 bounce, 1 sample per pass; press Tab for full render quality");

    let mut left_mouse_button_pressed = false;
    let mut right_mouse_button_pressed = false;
    let mut speed = 1.0;
    let mut move_queue: HashSet<gpu_camera::Direction> = HashSet::new();
    let mut dirty = true;
    let mut timing_window_start = Instant::now();
    let mut frame_time_total = Duration::ZERO;
    let mut frame_count = 0u32;
    let mut timing_pending_progress = false;
    window.request_redraw();

    event_loop.run(|event, control_handle| {
        control_handle.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => control_handle.exit(),
                WindowEvent::RedrawRequested => {
                    let frame_started = Instant::now();
                    let frame = match surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                        _ => {
                            window.request_redraw();
                            return;
                        }
                    };
                    let render_target = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    if dirty || !move_queue.is_empty() {
                        for direction in &move_queue {
                            camera.translate(*direction, speed * 0.1);
                        }
                        camera.update_uniforms();
                        renderer.reset_samples();
                    }
                    let tracing = renderer.needs_more_samples();
                    renderer.render_frame(&camera, &render_target);
                    dirty = false;
                    renderer.present(frame);
                    frame_time_total += frame_started.elapsed();
                    frame_count += 1;
                    timing_pending_progress |= tracing;
                    let finished = !renderer.needs_more_samples() && move_queue.is_empty();
                    if timing_window_start.elapsed() >= Duration::from_secs(1)
                        || (finished && timing_pending_progress)
                    {
                        let average_ms = frame_time_total.as_secs_f64() * 1000.0
                            / f64::from(frame_count.max(1));
                        let redraws_per_second = f64::from(frame_count)
                            / timing_window_start.elapsed().as_secs_f64().max(0.001);
                        println!(
                            "frame CPU work: {average_ms:.2} ms average ({redraws_per_second:.1} redraws/s), {} mode, pass {}",
                            if setup_mode { "setup" } else { "render" },
                            renderer.completed_passes(),
                        );
                        timing_window_start = Instant::now();
                        frame_time_total = Duration::ZERO;
                        frame_count = 0;
                        timing_pending_progress = false;
                    }
                    if renderer.needs_more_samples() || !move_queue.is_empty() {
                        window.request_redraw();
                    }
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseWheel { delta } => {
                    let delta = match delta {
                        MouseScrollDelta::PixelDelta(delta) => 0.01 * delta.y as f32,
                        MouseScrollDelta::LineDelta(_, y) => y,
                    };
                    if left_mouse_button_pressed {
                        camera.focus(delta, 0.1);
                    } else {
                        camera.zoom(delta, -1.0);
                    }
                    dirty = true;
                    window.request_redraw();
                }
                DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                    if left_mouse_button_pressed {
                        camera.update_lookat(dx as f32, dy as f32);
                        dirty = true;
                    }
                    if right_mouse_button_pressed {
                        camera.apeture(dx as f32, 0.001);
                        dirty = true;
                    }
                    if dirty {
                        window.request_redraw();
                    }
                }
                DeviceEvent::Button { button, state, .. } => {
                    let pressed = state == ElementState::Pressed;
                    match button {
                        0 => left_mouse_button_pressed = pressed,
                        1 => right_mouse_button_pressed = pressed,
                        _ => (),
                    }
                }
                DeviceEvent::Key(RawKeyEvent {
                    physical_key: key,
                    state,
                }) => {
                    let pressed = state == ElementState::Pressed;
                    let mode = match key {
                        PhysicalKey::Code(KeyCode::Digit1) => Some(0),
                        PhysicalKey::Code(KeyCode::Digit2) => Some(2),
                        PhysicalKey::Code(KeyCode::Digit3) => Some(1),
                        _ => None,
                    };
                    if pressed && key == PhysicalKey::Code(KeyCode::Tab) {
                        setup_mode = !setup_mode;
                        renderer.set_render_mode(0);
                        if setup_mode {
                            renderer.set_quality(1, 1);
                            println!("setup mode: 1 bounce, 1 sample per pass");
                        } else {
                            renderer.set_quality(args.bounces, args.samples);
                            println!(
                                "render mode: {} bounces, {} samples per pass",
                                args.bounces, args.samples
                            );
                        }
                        dirty = true;
                        window.request_redraw();
                    }
                    let direction = match key {
                        PhysicalKey::Code(KeyCode::KeyW) => Some(gpu_camera::Direction::Forward),
                        PhysicalKey::Code(KeyCode::KeyA) => Some(gpu_camera::Direction::Left),
                        PhysicalKey::Code(KeyCode::KeyS) => Some(gpu_camera::Direction::Backward),
                        PhysicalKey::Code(KeyCode::KeyD) => Some(gpu_camera::Direction::Right),
                        PhysicalKey::Code(KeyCode::Space) => Some(gpu_camera::Direction::Up),
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            Some(gpu_camera::Direction::Down)
                        }
                        _ => None,
                    };
                    if pressed && matches!(key, PhysicalKey::Code(KeyCode::Enter | KeyCode::KeyP)) {
                        match renderer.readback_colors() {
                            Ok(colors) => {
                                let timestamp = SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .map(|duration| duration.as_millis())
                                    .unwrap_or_default();
                                let filename = format!("output/{timestamp}.png");
                                if let Err(error) =
                                    io::save_colors_as_image(&colors, width, height, &filename)
                                {
                                    eprintln!("failed to save {filename}: {error}");
                                } else {
                                    println!("saved {filename}");
                                    control_handle.exit();
                                }
                            }
                            Err(error) => eprintln!("failed to read back rendered image: {error}"),
                        }
                    }
                    if key == PhysicalKey::Code(KeyCode::KeyF) && pressed {
                        filter_enabled = !filter_enabled;
                        renderer.set_filter_enabled(filter_enabled);
                        window.request_redraw();
                    }
                    if let Some(mode) = mode {
                        if pressed {
                            renderer.set_render_mode(mode);
                            dirty = true;
                            window.request_redraw();
                        }
                    } else if let Some(direction) = direction {
                        if pressed {
                            move_queue.insert(direction);
                        } else {
                            move_queue.remove(&direction);
                        }
                        window.request_redraw();
                    } else if key == PhysicalKey::Code(KeyCode::ShiftLeft) {
                        speed = if pressed { 3.0 } else { 1.0 };
                        if !move_queue.is_empty() {
                            window.request_redraw();
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    })?;
    Ok(())
}

fn make_default_setup(args: &Args) -> render::Renderer {
    let image = File::open("scene/hdr/studio_small.hdr")
        .ok()
        .and_then(|file| radiant::load(BufReader::new(file)).ok());
    let camera = camera::Camera::new(
        Vec3::new(13.0, 2.0, 3.0),
        Vec3::zero(),
        Vec3::unit_y(),
        20.0,
        args.width as f32 / args.height as f32,
        0.1,
        10.0,
    );
    render::Renderer {
        width: args.width,
        height: args.height,
        camera,
        world: Arc::new(io::random_scene(true, true, true, true, true)),
        sample_rate: args.samples,
        max_bounce: args.bounces,
        hdr: Arc::new(image),
        light_clamp: args.light_clamp,
    }
}

async fn connect_to_gpu(window: &Window) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Surface<'_>)> {
    use wgpu::TextureFormat::{Bgra8Unorm, Rgba8Unorm};

    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window)?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        })
        .await
        .context("failed to find a compatible adapter")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .context("failed to connect to the GPU")?;

    let capabilities = surface.get_capabilities(&adapter);
    let format = capabilities
        .formats
        .into_iter()
        .find(|format| matches!(format, Rgba8Unorm | Bgra8Unorm))
        .context("could not find a supported 8-bit surface format")?;
    let size = window.inner_size();
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        color_space: wgpu::SurfaceColorSpace::Auto,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 3,
    };
    surface.configure(&device, &config);
    Ok((device, queue, surface))
}
