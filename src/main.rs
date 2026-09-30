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
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

mod camera;
mod display;
mod gpu_camera;
mod gpu_render;
mod gpu_scene;
mod io;
mod material;
mod tracer;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    scene: Option<String>,
    #[arg(short, long, default_value_t = 128)]
    pub samples: u32,
    #[arg(short, long, default_value_t = 128)]
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

fn main() -> Result<()> {
    let args = Args::parse();
    let scene = if let Some(path) = &args.scene {
        io::load_scene(Path::new(path), &args)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
    } else {
        make_default_setup(&args)
    };
    let event_loop = EventLoop::new()?;
    let mut app = GpuApp::new(args, scene);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        anyhow::bail!(error);
    }
    Ok(())
}

struct GpuApp {
    args: Args,
    scene: Option<io::SceneData>,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    renderer: Option<gpu_render::PathTracer>,
    camera: Option<gpu_camera::Camera>,
    failure: Option<String>,
    left_mouse_button_pressed: bool,
    right_mouse_button_pressed: bool,
    filter_enabled: bool,
    setup_mode: bool,
    speed: f32,
    move_queue: HashSet<gpu_camera::Direction>,
    dirty: bool,
    timing_window_start: Instant,
    frame_time_total: Duration,
    frame_count: u32,
    timing_pending_progress: bool,
}

impl GpuApp {
    fn new(args: Args, scene: io::SceneData) -> Self {
        Self {
            filter_enabled: args.filter,
            args,
            scene: Some(scene),
            window: None,
            surface: None,
            renderer: None,
            camera: None,
            failure: None,
            left_mouse_button_pressed: false,
            right_mouse_button_pressed: false,
            setup_mode: true,
            speed: 1.0,
            move_queue: HashSet::new(),
            dirty: true,
            timing_window_start: Instant::now(),
            frame_time_total: Duration::ZERO,
            frame_count: 0,
            timing_pending_progress: false,
        }
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let width = self.args.width as u32;
        let height = self.args.height as u32;
        let window = Arc::new(
            event_loop.create_window(
                WindowAttributes::default()
                    .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
                    .with_resizable(false)
                    .with_title("GPU Path Tracer"),
            )?,
        );
        let scene = self.scene.take().context("scene was already initialized")?;
        let (eye, look_direction, fov, aspect_ratio, aperture, focus_dist) =
            scene.camera.gpu_parameters();
        let (device, queue, surface) = pollster::block_on(connect_to_gpu(window.clone()))?;
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
            &scene.gpu_scene,
            scene.hdr.as_ref(),
            self.args.bounces,
            self.args.samples,
            self.args.passes,
            self.args.gamma,
            self.args.light_clamp,
        );
        renderer.set_filter_enabled(self.filter_enabled);
        renderer.set_quality(1, 1);
        renderer.set_render_mode(2);
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

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.camera = Some(camera);
        self.window.as_ref().unwrap().request_redraw();
        println!("setup mode: unlit color/depth preview; press Tab for path tracing");
        Ok(())
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let (Some(surface), Some(camera), Some(renderer)) =
            (&self.surface, &mut self.camera, &mut self.renderer)
        else {
            return;
        };
        let frame_started = Instant::now();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                self.request_redraw();
                return;
            }
        };
        let render_target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if self.dirty || !self.move_queue.is_empty() {
            for direction in &self.move_queue {
                camera.translate(*direction, self.speed * 0.1);
            }
            camera.update_uniforms();
            renderer.reset_samples();
        }
        let tracing = renderer.needs_more_samples();
        renderer.render_frame(camera, &render_target);
        self.dirty = false;
        renderer.present(frame);
        self.frame_time_total += frame_started.elapsed();
        self.frame_count += 1;
        self.timing_pending_progress |= tracing;
        let finished = !renderer.needs_more_samples() && self.move_queue.is_empty();
        if self.timing_window_start.elapsed() >= Duration::from_secs(1)
            || (finished && self.timing_pending_progress)
        {
            let average_ms =
                self.frame_time_total.as_secs_f64() * 1000.0 / f64::from(self.frame_count.max(1));
            let redraws_per_second = f64::from(self.frame_count)
                / self.timing_window_start.elapsed().as_secs_f64().max(0.001);
            println!(
                "frame CPU work: {average_ms:.2} ms average ({redraws_per_second:.1} redraws/s), {} mode, pass {}",
                if self.setup_mode { "setup" } else { "render" },
                renderer.completed_passes(),
            );
            self.timing_window_start = Instant::now();
            self.frame_time_total = Duration::ZERO;
            self.frame_count = 0;
            self.timing_pending_progress = false;
        }
        if renderer.needs_more_samples() || !self.move_queue.is_empty() {
            self.request_redraw();
        }
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: PhysicalKey, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        let window = self.window.clone();
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let mode = match key {
            PhysicalKey::Code(KeyCode::Digit1) => Some(0),
            PhysicalKey::Code(KeyCode::Digit2) => Some(2),
            PhysicalKey::Code(KeyCode::Digit3) => Some(1),
            _ => None,
        };
        if pressed && key == PhysicalKey::Code(KeyCode::Tab) {
            self.setup_mode = !self.setup_mode;
            if self.setup_mode {
                renderer.set_quality(1, 1);
                renderer.set_render_mode(2);
                println!("setup mode: unlit color/depth preview");
            } else {
                renderer.set_render_mode(0);
                renderer.set_quality(self.args.bounces, self.args.samples);
                println!(
                    "render mode: {} bounces, {} samples per pass",
                    self.args.bounces, self.args.samples
                );
            }
            self.dirty = true;
            if let Some(window) = &window {
                window.request_redraw();
            }
        }

        let direction = match key {
            PhysicalKey::Code(KeyCode::KeyW) => Some(gpu_camera::Direction::Forward),
            PhysicalKey::Code(KeyCode::KeyA) => Some(gpu_camera::Direction::Left),
            PhysicalKey::Code(KeyCode::KeyS) => Some(gpu_camera::Direction::Backward),
            PhysicalKey::Code(KeyCode::KeyD) => Some(gpu_camera::Direction::Right),
            PhysicalKey::Code(KeyCode::Space) => Some(gpu_camera::Direction::Up),
            PhysicalKey::Code(KeyCode::ControlLeft) => Some(gpu_camera::Direction::Down),
            _ => None,
        };
        if pressed && matches!(key, PhysicalKey::Code(KeyCode::Enter | KeyCode::KeyP)) {
            let width = self.args.width as u32;
            let height = self.args.height as u32;
            match renderer.readback_colors() {
                Ok(colors) => {
                    let timestamp = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|duration| duration.as_millis())
                        .unwrap_or_default();
                    let filename = format!("output/{timestamp}.png");
                    if let Err(error) = io::save_colors_as_image(&colors, width, height, &filename)
                    {
                        eprintln!("failed to save {filename}: {error}");
                    } else {
                        println!("saved {filename}");
                        event_loop.exit();
                    }
                }
                Err(error) => eprintln!("failed to read back rendered image: {error}"),
            }
        }
        if key == PhysicalKey::Code(KeyCode::KeyF) && pressed {
            self.filter_enabled = !self.filter_enabled;
            renderer.set_filter_enabled(self.filter_enabled);
            if let Some(window) = &window {
                window.request_redraw();
            }
        }
        if let Some(mode) = mode {
            if pressed {
                renderer.set_render_mode(mode);
                self.dirty = true;
                if let Some(window) = &window {
                    window.request_redraw();
                }
            }
        } else if let Some(direction) = direction {
            if pressed {
                self.move_queue.insert(direction);
            } else {
                self.move_queue.remove(&direction);
            }
            if let Some(window) = &window {
                window.request_redraw();
            }
        } else if key == PhysicalKey::Code(KeyCode::ShiftLeft) {
            self.speed = if pressed { 3.0 } else { 1.0 };
            if !self.move_queue.is_empty() {
                if let Some(window) = &window {
                    window.request_redraw();
                }
            }
        }
    }
}

impl ApplicationHandler for GpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.failure.is_some() {
            return;
        }
        if let Err(error) = self.initialize(event_loop) {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Tab) && event.repeat {
                    return;
                }
                self.handle_key(event_loop, event.physical_key, event.state);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.left_mouse_button_pressed = pressed,
                    MouseButton::Right => self.right_mouse_button_pressed = pressed,
                    _ => (),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::PixelDelta(delta) => 0.01 * delta.y as f32,
                    MouseScrollDelta::LineDelta(_, y) => y,
                };
                if let Some(camera) = &mut self.camera {
                    if self.left_mouse_button_pressed {
                        camera.focus(delta, 0.1);
                    } else {
                        camera.zoom(delta, -1.0);
                    }
                }
                self.dirty = true;
                self.request_redraw();
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if let Some(camera) = &mut self.camera {
                if self.left_mouse_button_pressed {
                    camera.update_lookat(dx as f32, dy as f32);
                    self.dirty = true;
                }
                if self.right_mouse_button_pressed {
                    camera.apeture(dx as f32, 0.001);
                    self.dirty = true;
                }
            }
            if self.dirty {
                self.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn make_default_setup(args: &Args) -> io::SceneData {
    let image = File::open("scene/hdr/studio_small.hdr")
        .ok()
        .and_then(|file| radiant::load(BufReader::new(file)).ok());
    let camera = camera::Camera::new(
        Vec3::new(13.0, 2.0, 3.0),
        Vec3::zero(),
        20.0,
        args.width as f32 / args.height as f32,
        0.1,
        10.0,
    );
    io::SceneData {
        camera,
        gpu_scene: io::random_scene(true, true, true, true, true),
        hdr: image,
    }
}

async fn connect_to_gpu(
    window: Arc<Window>,
) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Surface<'static>)> {
    use wgpu::TextureFormat::{Bgra8Unorm, Rgba8Unorm};

    let instance = wgpu::Instance::default();
    let size = window.inner_size();
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
