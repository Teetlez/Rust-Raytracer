use crate::{
    gpu_camera::{Camera, CameraUniforms},
    gpu_scene::{GpuBvhNode, GpuMaterial, GpuPrimitive, GpuScene},
};
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use ultraviolet::Vec3;
use wgpu::{util::DeviceExt, PipelineCompilationOptions};

const FILTER_UPDATE_INTERVAL: u32 = 8;

pub struct PathTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    sample_sums: wgpu::Buffer,
    filter_a: wgpu::Buffer,
    _hdr_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    trace_pipeline: wgpu::ComputePipeline,
    filter_pipelines: [wgpu::ComputePipeline; 3],
    display_pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
    max_passes: u32,
    filter_dirty: bool,
    last_filtered_pass: u32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Uniforms {
    camera: CameraUniforms,
    width: u32,
    height: u32,
    frame_num: u32,
    max_bounces: u32,
    samples_per_pass: u32,
    render_mode: u32,
    hdr_width: u32,
    hdr_height: u32,
    hdr_enabled: u32,
    filter_enabled: u32,
    gamma: f32,
    light_clamp: f32,
    filter_valid: u32,
    reset_generation: u32,
    _pad1: [u32; 2],
}

impl PathTracer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        scene: &GpuScene,
        hdr: Option<&radiant::Image>,
        max_bounces: u32,
        samples_per_pass: u32,
        max_passes: u32,
        gamma: f32,
        light_clamp: f32,
    ) -> Self {
        device.on_uncaptured_error(Arc::new(|error| {
            panic!("Aborting due to an error: {}", error);
        }));

        let (hdr_texture, hdr_view, hdr_width, hdr_height) =
            create_hdr_texture(&device, &queue, hdr);
        let sample_sums = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("accumulated radiance"),
            size: (u64::from(width) * u64::from(height) * 16).max(16),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let variance_stats = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("per-pixel variance statistics"),
            size: (u64::from(width) * u64::from(height) * 16).max(16),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let filter_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bilateral filter output A"),
            size: (u64::from(width) * u64::from(height) * 16).max(16),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let filter_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bilateral filter output B"),
            size: (u64::from(width) * u64::from(height) * 16).max(16),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let nodes = create_storage_buffer(&device, "BVH nodes", &scene.nodes);
        let primitives = create_storage_buffer(&device, "scene primitives", &scene.primitives);
        let materials = create_storage_buffer(&device, "scene materials", &scene.materials);

        let uniforms = Uniforms {
            camera: CameraUniforms::zeroed(),
            width,
            height,
            frame_num: 0,
            max_bounces,
            samples_per_pass: samples_per_pass.max(1),
            render_mode: 0,
            hdr_width,
            hdr_height,
            hdr_enabled: u32::from(hdr.is_some()),
            filter_enabled: 0,
            gamma: gamma.recip(),
            light_clamp,
            filter_valid: 0,
            reset_generation: 1,
            _pad1: [0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("path tracer uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU path tracer"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders.wgsl")).into(),
            ),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("path tracer bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_layout_entry(2),
                storage_layout_entry(3),
                storage_layout_entry(4),
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                storage_rw_layout_entry(
                    6,
                    wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                ),
                storage_rw_layout_entry(7, wgpu::ShaderStages::COMPUTE),
                storage_rw_layout_entry(8, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("path tracer resources"),
            layout: &bind_group_layout,
            entries: &[
                buffer_entry(0, &uniform_buffer),
                buffer_entry(1, &sample_sums),
                buffer_entry(2, &nodes),
                buffer_entry(3, &primitives),
                buffer_entry(4, &materials),
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                buffer_entry(6, &filter_a),
                buffer_entry(7, &filter_b),
                buffer_entry(8, &variance_stats),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("path tracer pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let trace_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("path trace compute"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("trace_compute"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });
        let filter_pipelines = [
            create_compute_pipeline(&device, &pipeline_layout, &shader, "filter_pass_1"),
            create_compute_pipeline(&device, &pipeline_layout, &shader, "filter_pass_2"),
            create_compute_pipeline(&device, &pipeline_layout, &shader, "filter_pass_3"),
        ];
        let display_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("path tracer display"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("display_vs"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("display_fs"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            device,
            queue,
            uniforms,
            uniform_buffer,
            sample_sums,
            filter_a,
            _hdr_texture: hdr_texture,
            bind_group,
            trace_pipeline,
            filter_pipelines,
            display_pipeline,
            width,
            height,
            max_passes: max_passes.max(1),
            filter_dirty: false,
            last_filtered_pass: 0,
        }
    }

    pub fn reset_samples(&mut self) {
        self.uniforms.frame_num = 0;
        self.uniforms.filter_valid = 0;
        self.uniforms.reset_generation = self.uniforms.reset_generation.wrapping_add(1).max(1);
        self.last_filtered_pass = 0;
    }

    pub fn set_render_mode(&mut self, mode: u32) {
        let mode = mode.min(2);
        if self.uniforms.render_mode != mode {
            self.uniforms.render_mode = mode;
            self.reset_samples();
        }
    }

    pub fn set_quality(&mut self, max_bounces: u32, samples_per_pass: u32) {
        let samples_per_pass = samples_per_pass.max(1);
        if self.uniforms.max_bounces != max_bounces
            || self.uniforms.samples_per_pass != samples_per_pass
        {
            self.uniforms.max_bounces = max_bounces;
            self.uniforms.samples_per_pass = samples_per_pass;
            self.reset_samples();
        }
    }

    pub fn needs_more_samples(&self) -> bool {
        self.uniforms.frame_num < self.max_passes
    }

    pub fn completed_passes(&self) -> u32 {
        self.uniforms.frame_num
    }

    pub fn set_filter_enabled(&mut self, enabled: bool) {
        let enabled = u32::from(enabled);
        if self.uniforms.filter_enabled != enabled {
            self.uniforms.filter_enabled = enabled;
            self.uniforms.filter_valid = 0;
            self.filter_dirty = true;
        }
    }

    pub fn readback_colors(&self) -> Result<Vec<u32>> {
        let byte_len = (u64::from(self.width) * u64::from(self.height) * 16).max(16);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image readback"),
            size: byte_len,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("copy image for PNG export"),
            });
        let source = if self.uniforms.filter_enabled == 0 || self.uniforms.filter_valid == 0 {
            &self.sample_sums
        } else {
            &self.filter_a
        };
        encoder.copy_buffer_to_buffer(source, 0, &readback, 0, byte_len);
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("failed while waiting for image readback")?;
        receiver
            .recv()
            .context("image readback callback was dropped")?
            .context("failed to map image readback buffer")?;

        let mapped = slice
            .get_mapped_range()
            .context("failed to access mapped image buffer")?;
        let colors = bytemuck::cast_slice::<u8, f32>(&mapped)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| {
                let color = Vec3::new(pixel[0], pixel[1], pixel[2]);
                crate::render::to_rgb(&color, self.uniforms.gamma)
            })
            .collect();
        drop(mapped);
        readback.unmap();
        Ok(colors)
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }

    pub fn render_frame(&mut self, camera: &Camera, target: &wgpu::TextureView) {
        self.uniforms.camera = *camera.uniforms();
        let should_trace = self.uniforms.frame_num < self.max_passes;
        if should_trace {
            self.uniforms.frame_num += 1;
        }
        let should_filter = self.uniforms.filter_enabled != 0
            && (self.filter_dirty
                || (self.uniforms.frame_num != self.last_filtered_pass
                    && (self.uniforms.frame_num % FILTER_UPDATE_INTERVAL == 0
                        || (should_trace && self.uniforms.frame_num >= self.max_passes))));
        if should_filter {
            self.uniforms.filter_valid = 1;
            self.filter_dirty = false;
            self.last_filtered_pass = self.uniforms.frame_num;
        }
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("path trace frame"),
            });
        if should_trace {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("trace samples"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.trace_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
        if should_filter {
            for pipeline in &self.filter_pipelines {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bilateral filter"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("display accumulated image"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.display_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
    }
}

fn storage_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_rw_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn create_storage_buffer<T: Pod>(device: &wgpu::Device, label: &str, data: &[T]) -> wgpu::Buffer {
    if data.is_empty() {
        let placeholder = [0u32; 4];
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&placeholder),
            usage: wgpu::BufferUsages::STORAGE,
        })
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE,
        })
    }
}

fn create_hdr_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: Option<&radiant::Image>,
) -> (wgpu::Texture, wgpu::TextureView, u32, u32) {
    let (width, height) = image
        .map(|image| (image.width as u32, image.height as u32))
        .unwrap_or((1, 1));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR environment"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    if let Some(image) = image {
        for pixel in &image.data {
            rgba.extend_from_slice(&[pixel.r, pixel.g, pixel.b, 1.0]);
        }
    } else {
        rgba.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
    }
    let row_bytes = width * 16;
    let padded_row_bytes = row_bytes.div_ceil(256) * 256;
    let source = bytemuck::cast_slice::<f32, u8>(&rgba);
    let mut padded = vec![0u8; (padded_row_bytes * height) as usize];
    for row in 0..height as usize {
        let source_start = row * row_bytes as usize;
        let target_start = row * padded_row_bytes as usize;
        padded[target_start..target_start + row_bytes as usize]
            .copy_from_slice(&source[source_start..source_start + row_bytes as usize]);
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &padded,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(padded_row_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view, width, height)
}

const _: () = {
    assert!(std::mem::size_of::<GpuPrimitive>() == 112);
    assert!(std::mem::size_of::<GpuMaterial>() == 48);
    assert!(std::mem::size_of::<GpuBvhNode>() == 48);
    assert!(std::mem::size_of::<Uniforms>() == 192);
};

#[cfg(test)]
mod tests {
    use naga::{valid::Capabilities, valid::ValidationFlags, valid::Validator};

    #[test]
    fn path_tracer_shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("shaders.wgsl"))
            .expect("path tracer WGSL should parse");
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("path tracer WGSL should validate");
        let uniform_size = module
            .types
            .iter()
            .find_map(|(_, ty)| {
                if ty.name.as_deref() != Some("Uniforms") {
                    return None;
                }
                match &ty.inner {
                    naga::TypeInner::Struct { span, .. } => Some(*span as usize),
                    _ => None,
                }
            })
            .expect("WGSL Uniforms type should exist");
        assert_eq!(uniform_size, std::mem::size_of::<super::Uniforms>());
    }
}
