use std::{collections::HashMap, path::Path, sync::mpsc};

use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{Camera2d, Color, GpuInfo, Rect, RenderStats, Renderer, Vec2};

const RECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const IMAGE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.tint = input.tint;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, input.uv) * input.tint;
}
"#;

const INITIAL_VERTEX_BUFFER_BYTES: wgpu::BufferAddress = 64 * 1024;
const TIMESTAMP_QUERY_COUNT: u32 = 2;
const TIMESTAMP_QUERY_BUFFER_BYTES: wgpu::BufferAddress = TIMESTAMP_QUERY_COUNT
    as wgpu::BufferAddress
    * std::mem::size_of::<u64>() as wgpu::BufferAddress;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl RectVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImageVertex {
    position: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
}

impl ImageVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Clone, Copy)]
struct RectCommand {
    camera: Camera2d,
    rect: Rect,
    color: Color,
}

#[derive(Clone)]
struct ImageCommand {
    camera: Camera2d,
    texture_id: String,
    rect: Rect,
    source: Option<Rect>,
    tint: Color,
    flip_x: bool,
    rotation: i32,
}

enum RenderCommand {
    Rect(RectCommand),
    Image(ImageCommand),
}

enum PreparedCommand {
    Rect {
        vertex_offset: wgpu::BufferAddress,
        vertex_count: u32,
    },
    Image {
        texture_id: String,
        vertex_offset: wgpu::BufferAddress,
        vertex_count: u32,
    },
}

enum PendingBatch {
    Rect(Vec<RectVertex>),
    Image {
        texture_id: String,
        vertices: Vec<ImageVertex>,
    },
}

struct GpuTexture {
    _texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    size: Vec2,
}

struct DynamicVertexBuffer {
    label: &'static str,
    buffer: wgpu::Buffer,
    capacity: wgpu::BufferAddress,
}

impl DynamicVertexBuffer {
    fn new(device: &wgpu::Device, label: &'static str, capacity: wgpu::BufferAddress) -> Self {
        let capacity = capacity.max(1);
        Self {
            label,
            buffer: create_vertex_buffer(device, label, capacity),
            capacity,
        }
    }

    fn write<T>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[T]) -> bool
    where
        T: Pod,
    {
        let bytes = bytemuck::cast_slice(vertices);
        if bytes.is_empty() {
            return false;
        }

        let required = bytes.len() as wgpu::BufferAddress;
        if required > self.capacity {
            self.capacity = required
                .next_power_of_two()
                .max(INITIAL_VERTEX_BUFFER_BYTES);
            self.buffer = create_vertex_buffer(device, self.label, self.capacity);
        }

        queue.write_buffer(&self.buffer, 0, bytes);
        true
    }
}

struct GpuProfiler {
    enabled: bool,
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    pending_readback: Option<mpsc::Receiver<bool>>,
    timestamp_period: f32,
    last_frame_ms: Option<f32>,
}

impl GpuProfiler {
    fn new(device: &wgpu::Device, timestamp_period: f32, enabled: bool) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Frame Timestamp Query Set"),
            ty: wgpu::QueryType::Timestamp,
            count: TIMESTAMP_QUERY_COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Timestamp Resolve Buffer"),
            size: TIMESTAMP_QUERY_BUFFER_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Timestamp Readback Buffer"),
            size: TIMESTAMP_QUERY_BUFFER_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            enabled,
            query_set,
            resolve_buffer,
            readback_buffer,
            pending_readback: None,
            timestamp_period,
            last_frame_ms: None,
        }
    }

    fn can_capture_frame(&self) -> bool {
        self.enabled && self.pending_readback.is_none()
    }

    fn register_readback(&mut self, command_buffer: &wgpu::CommandBuffer) {
        if !self.can_capture_frame() {
            return;
        }

        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &self.readback_buffer,
            wgpu::MapMode::Read,
            ..,
            move |result| {
                let _ = sender.send(result.is_ok());
            },
        );
        self.pending_readback = Some(receiver);
    }

    fn collect_pending_readback(&mut self) {
        let Some(receiver) = self.pending_readback.as_ref() else {
            return;
        };

        let readback_ready = match receiver.try_recv() {
            Ok(success) => Some(success),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(false),
        };

        let Some(success) = readback_ready else {
            return;
        };

        self.pending_readback = None;
        if !success {
            return;
        }

        {
            let data = self.readback_buffer.slice(..).get_mapped_range();
            let timestamps = bytemuck::cast_slice::<u8, u64>(&data);
            if timestamps.len() >= 2 && timestamps[1] >= timestamps[0] {
                let elapsed_ns = (timestamps[1] - timestamps[0]) as f32 * self.timestamp_period;
                self.last_frame_ms = Some(elapsed_ns / 1_000_000.0);
            }
        }
        self.readback_buffer.unmap();
    }
}

pub struct WgpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    rect_pipeline: wgpu::RenderPipeline,
    image_pipeline: wgpu::RenderPipeline,
    image_bind_group_layout: wgpu::BindGroupLayout,
    camera: Camera2d,
    clear_color: Color,
    commands: Vec<RenderCommand>,
    textures: HashMap<String, GpuTexture>,
    rect_vertices: DynamicVertexBuffer,
    image_vertices: DynamicVertexBuffer,
    gpu_info: GpuInfo,
    gpu_profiler: Option<GpuProfiler>,
    last_frame_stats: RenderStats,
}

impl WgpuRenderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        // The window outlives the renderer inside the app event loop.
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_display_and_window(window, window)
                .context("failed to read native window handles")?;
            instance
                .create_surface_unsafe(target)
                .context("failed to create wgpu surface")?
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("no compatible GPU adapter found")?;
        let adapter_info = adapter.get_info();
        let supported_features = adapter.features();
        let timestamp_query_available =
            supported_features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let required_features = if timestamp_query_available {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Alien Archive Device"),
                required_features,
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .context("failed to create GPU device")?;
        let device_limits = device.limits();
        let gpu_info = GpuInfo {
            name: adapter_info.name,
            backend: format!("{:?}", adapter_info.backend),
            device_type: format!("{:?}", adapter_info.device_type),
            driver: adapter_info.driver,
            driver_info: adapter_info.driver_info,
            enabled_features: feature_summary(required_features),
            supported_features: feature_summary(supported_features),
            max_texture_dimension_2d: device_limits.max_texture_dimension_2d,
            max_bind_groups: device_limits.max_bind_groups,
            timestamp_query: timestamp_query_available,
        };
        eprintln!(
            "Alien Archive GPU: {} backend={} type={} driver={} features={} limits=max_texture_2d:{} max_bind_groups:{}",
            gpu_info.name,
            gpu_info.backend,
            gpu_info.device_type,
            gpu_info.driver,
            gpu_info.enabled_features,
            gpu_info.max_texture_dimension_2d,
            gpu_info.max_bind_groups
        );
        let timestamp_profiling_enabled =
            std::env::var_os("ALIEN_ARCHIVE_GPU_TIMESTAMPS").is_some();
        let gpu_profiler = if timestamp_query_available {
            Some(GpuProfiler::new(
                &device,
                queue.get_timestamp_period(),
                timestamp_profiling_enabled,
            ))
        } else {
            None
        };

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);
        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(surface_caps.present_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rectangle Shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });
        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rectangle Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rectangle Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let image_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Image Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Image Shader"),
            source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER.into()),
        });
        let image_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Image Pipeline Layout"),
                bind_group_layouts: &[Some(&image_bind_group_layout)],
                immediate_size: 0,
            });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Image Pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &image_shader,
                entry_point: Some("vs_main"),
                buffers: &[ImageVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &image_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let rect_vertices = DynamicVertexBuffer::new(
            &device,
            "Shared Rectangle Vertex Buffer",
            INITIAL_VERTEX_BUFFER_BYTES,
        );
        let image_vertices = DynamicVertexBuffer::new(
            &device,
            "Shared Image Vertex Buffer",
            INITIAL_VERTEX_BUFFER_BYTES,
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            rect_pipeline,
            image_pipeline,
            image_bind_group_layout,
            camera: Camera2d::default(),
            clear_color: Color::rgb(0.015, 0.019, 0.035),
            commands: Vec::with_capacity(256),
            textures: HashMap::new(),
            rect_vertices,
            image_vertices,
            gpu_info,
            gpu_profiler,
            last_frame_stats: RenderStats::default(),
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn begin_frame(&mut self, camera: Camera2d) {
        self.camera = camera;
        self.commands.clear();
    }

    pub fn finish_frame(&mut self) -> Result<()> {
        self.collect_gpu_profile();

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => bail!("GPU surface validation failed"),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let prepared_commands = self.prepare_commands();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let capture_timestamps = self
            .gpu_profiler
            .as_ref()
            .is_some_and(GpuProfiler::can_capture_frame);
        let timestamp_writes = self.gpu_profiler.as_ref().and_then(|profiler| {
            capture_timestamps.then_some(wgpu::RenderPassTimestampWrites {
                query_set: &profiler.query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            })
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(to_wgpu_color(self.clear_color)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes,
                multiview_mask: None,
            });

            for command in &prepared_commands {
                match command {
                    PreparedCommand::Rect {
                        vertex_offset,
                        vertex_count,
                    } => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.set_vertex_buffer(
                            0,
                            self.rect_vertices.buffer.slice(*vertex_offset..),
                        );
                        pass.draw(0..*vertex_count, 0..1);
                    }
                    PreparedCommand::Image {
                        texture_id,
                        vertex_offset,
                        vertex_count,
                    } => {
                        let Some(texture) = self.textures.get(texture_id) else {
                            continue;
                        };

                        pass.set_pipeline(&self.image_pipeline);
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.set_vertex_buffer(
                            0,
                            self.image_vertices.buffer.slice(*vertex_offset..),
                        );
                        pass.draw(0..*vertex_count, 0..1);
                    }
                }
            }
        }

        if capture_timestamps && let Some(profiler) = &self.gpu_profiler {
            encoder.resolve_query_set(
                &profiler.query_set,
                0..TIMESTAMP_QUERY_COUNT,
                &profiler.resolve_buffer,
                0,
            );
            encoder.copy_buffer_to_buffer(
                &profiler.resolve_buffer,
                0,
                &profiler.readback_buffer,
                0,
                TIMESTAMP_QUERY_BUFFER_BYTES,
            );
        }

        let command_buffer = encoder.finish();
        if capture_timestamps && let Some(profiler) = &mut self.gpu_profiler {
            profiler.register_readback(&command_buffer);
        }

        self.queue.submit(std::iter::once(command_buffer));
        self.collect_gpu_profile();
        frame.present();

        Ok(())
    }

    fn prepare_commands(&mut self) -> Vec<PreparedCommand> {
        let mut prepared = Vec::with_capacity(self.commands.len());
        let mut pending: Option<PendingBatch> = None;
        let mut rect_upload = Vec::<RectVertex>::new();
        let mut image_upload = Vec::<ImageVertex>::new();
        let mut stats = RenderStats {
            queued_commands: self.commands.len(),
            loaded_textures: self.textures.len(),
            gpu_info: self.gpu_info.clone(),
            gpu_frame_ms: self
                .gpu_profiler
                .as_ref()
                .and_then(|profiler| profiler.last_frame_ms),
            ..RenderStats::default()
        };

        for command in &self.commands {
            match command {
                RenderCommand::Rect(command) => {
                    stats.rect_commands += 1;
                    let vertices = self.build_rect_vertices(*command);
                    match &mut pending {
                        Some(PendingBatch::Rect(batch_vertices)) => {
                            batch_vertices.extend_from_slice(&vertices);
                        }
                        _ => {
                            Self::flush_pending_batch(
                                &mut pending,
                                &mut prepared,
                                &mut rect_upload,
                                &mut image_upload,
                                &mut stats,
                            );
                            pending = Some(PendingBatch::Rect(vertices.to_vec()));
                        }
                    }
                }
                RenderCommand::Image(command) => {
                    stats.image_commands += 1;
                    if command.texture_id.starts_with("__map_ground_cache_") {
                        stats.ground_chunk_commands += 1;
                    }

                    if !self.textures.contains_key(&command.texture_id) {
                        stats.skipped_image_commands += 1;
                        continue;
                    }

                    let Some(texture) = self.textures.get(&command.texture_id) else {
                        continue;
                    };

                    let vertices = self.build_image_vertices(command, texture.size);
                    match &mut pending {
                        Some(PendingBatch::Image {
                            texture_id,
                            vertices: batch_vertices,
                        }) if texture_id == &command.texture_id => {
                            batch_vertices.extend_from_slice(&vertices);
                        }
                        _ => {
                            Self::flush_pending_batch(
                                &mut pending,
                                &mut prepared,
                                &mut rect_upload,
                                &mut image_upload,
                                &mut stats,
                            );
                            pending = Some(PendingBatch::Image {
                                texture_id: command.texture_id.clone(),
                                vertices: vertices.to_vec(),
                            });
                        }
                    }
                }
            }
        }

        Self::flush_pending_batch(
            &mut pending,
            &mut prepared,
            &mut rect_upload,
            &mut image_upload,
            &mut stats,
        );
        stats.draw_calls = prepared.len();
        self.upload_frame_vertices(&rect_upload, &image_upload, &mut stats);
        self.last_frame_stats = stats;

        prepared
    }

    fn collect_gpu_profile(&mut self) {
        let Some(profiler) = &mut self.gpu_profiler else {
            return;
        };
        if !profiler.enabled {
            return;
        }

        let _ = self.device.poll(wgpu::PollType::Poll);
        profiler.collect_pending_readback();
        self.last_frame_stats.gpu_frame_ms = profiler.last_frame_ms;
    }

    fn flush_pending_batch(
        pending: &mut Option<PendingBatch>,
        prepared: &mut Vec<PreparedCommand>,
        rect_upload: &mut Vec<RectVertex>,
        image_upload: &mut Vec<ImageVertex>,
        stats: &mut RenderStats,
    ) {
        let Some(batch) = pending.take() else {
            return;
        };

        match batch {
            PendingBatch::Rect(vertices) => {
                if vertices.is_empty() {
                    return;
                }
                let vertex_offset = byte_len(rect_upload);
                let vertex_count = vertices.len() as u32;
                rect_upload.extend_from_slice(&vertices);
                stats.rect_batches += 1;
                prepared.push(PreparedCommand::Rect {
                    vertex_offset,
                    vertex_count,
                });
            }
            PendingBatch::Image {
                texture_id,
                vertices,
            } => {
                if vertices.is_empty() {
                    return;
                }
                let vertex_offset = byte_len(image_upload);
                let vertex_count = vertices.len() as u32;
                image_upload.extend_from_slice(&vertices);
                stats.image_batches += 1;
                prepared.push(PreparedCommand::Image {
                    texture_id,
                    vertex_offset,
                    vertex_count,
                });
            }
        }
    }

    fn upload_frame_vertices(
        &mut self,
        rect_vertices: &[RectVertex],
        image_vertices: &[ImageVertex],
        stats: &mut RenderStats,
    ) {
        if self
            .rect_vertices
            .write(&self.device, &self.queue, rect_vertices)
        {
            stats.vertex_buffers += 1;
        }
        if self
            .image_vertices
            .write(&self.device, &self.queue, image_vertices)
        {
            stats.vertex_buffers += 1;
        }
    }

    fn build_rect_vertices(&self, command: RectCommand) -> [RectVertex; 6] {
        let rect = command.rect;
        let color = [
            command.color.r,
            command.color.g,
            command.color.b,
            command.color.a,
        ];

        let top_left = self.world_to_clip(command.camera, rect.origin);
        let top_right = self.world_to_clip(command.camera, Vec2::new(rect.right(), rect.origin.y));
        let bottom_right =
            self.world_to_clip(command.camera, Vec2::new(rect.right(), rect.bottom()));
        let bottom_left =
            self.world_to_clip(command.camera, Vec2::new(rect.origin.x, rect.bottom()));

        [
            RectVertex {
                position: top_left,
                color,
            },
            RectVertex {
                position: bottom_left,
                color,
            },
            RectVertex {
                position: bottom_right,
                color,
            },
            RectVertex {
                position: top_left,
                color,
            },
            RectVertex {
                position: bottom_right,
                color,
            },
            RectVertex {
                position: top_right,
                color,
            },
        ]
    }

    fn build_image_vertices(&self, command: &ImageCommand, texture_size: Vec2) -> [ImageVertex; 6] {
        let rect = command.rect;
        let tint = [
            command.tint.r,
            command.tint.g,
            command.tint.b,
            command.tint.a,
        ];
        let source = command
            .source
            .unwrap_or_else(|| Rect::new(Vec2::ZERO, texture_size));
        let mut uv_left = source.origin.x / texture_size.x;
        let uv_top = source.origin.y / texture_size.y;
        let mut uv_right = source.right() / texture_size.x;
        let uv_bottom = source.bottom() / texture_size.y;
        if command.flip_x {
            std::mem::swap(&mut uv_left, &mut uv_right);
        }

        let [top_left, top_right, bottom_right, bottom_left] =
            transformed_rect_points(rect, command.rotation);
        let top_left = self.world_to_clip(command.camera, top_left);
        let top_right = self.world_to_clip(command.camera, top_right);
        let bottom_right = self.world_to_clip(command.camera, bottom_right);
        let bottom_left = self.world_to_clip(command.camera, bottom_left);

        [
            ImageVertex {
                position: top_left,
                uv: [uv_left, uv_top],
                tint,
            },
            ImageVertex {
                position: bottom_left,
                uv: [uv_left, uv_bottom],
                tint,
            },
            ImageVertex {
                position: bottom_right,
                uv: [uv_right, uv_bottom],
                tint,
            },
            ImageVertex {
                position: top_left,
                uv: [uv_left, uv_top],
                tint,
            },
            ImageVertex {
                position: bottom_right,
                uv: [uv_right, uv_bottom],
                tint,
            },
            ImageVertex {
                position: top_right,
                uv: [uv_right, uv_top],
                tint,
            },
        ]
    }

    fn world_to_clip(&self, camera: Camera2d, point: Vec2) -> [f32; 2] {
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let relative = (point - camera.position) * camera.zoom;
        let screen = Vec2::new(relative.x + width * 0.5, relative.y + height * 0.5);

        [
            (screen.x / width) * 2.0 - 1.0,
            1.0 - (screen.y / height) * 2.0,
        ]
    }
}

impl Renderer for WgpuRenderer {
    fn load_texture(&mut self, id: &str, path: &Path) -> Result<()> {
        let image = image::ImageReader::open(path)
            .with_context(|| format!("failed to open image {}", path.display()))?
            .decode()
            .with_context(|| format!("failed to decode image {}", path.display()))?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        self.load_texture_rgba(id, width, height, rgba.as_raw())
    }

    fn load_texture_rgba(&mut self, id: &str, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        if width == 0 || height == 0 {
            bail!("texture {id} has empty dimensions");
        }

        let expected_len = width as usize * height as usize * 4;
        if rgba.len() != expected_len {
            bail!(
                "texture {id} expected {expected_len} RGBA bytes, got {}",
                rgba.len()
            );
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(id),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Image Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Image Bind Group"),
            layout: &self.image_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.textures.insert(
            id.to_owned(),
            GpuTexture {
                _texture: texture,
                _sampler: sampler,
                bind_group,
                size: Vec2::new(width as f32, height as f32),
            },
        );

        Ok(())
    }

    fn texture_size(&self, id: &str) -> Option<Vec2> {
        self.textures.get(id).map(|texture| texture.size)
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(self.config.width as f32, self.config.height as f32)
    }

    fn frame_stats(&self) -> RenderStats {
        self.last_frame_stats.clone()
    }

    fn visible_world_rect(&self) -> Rect {
        let zoom = self.camera.zoom.max(0.001);
        let size = Vec2::new(
            self.config.width as f32 / zoom,
            self.config.height as f32 / zoom,
        );
        Rect::new(
            Vec2::new(
                self.camera.position.x - size.x * 0.5,
                self.camera.position.y - size.y * 0.5,
            ),
            size,
        )
    }

    fn set_camera(&mut self, camera: Camera2d) {
        self.camera = camera;
    }

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(RenderCommand::Rect(RectCommand {
            camera: self.camera,
            rect,
            color,
        }));
    }

    fn draw_image(&mut self, texture_id: &str, rect: Rect, tint: Color) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            camera: self.camera,
            texture_id: texture_id.to_owned(),
            rect,
            source: None,
            tint,
            flip_x: false,
            rotation: 0,
        }));
    }

    fn draw_image_transformed(
        &mut self,
        texture_id: &str,
        rect: Rect,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            camera: self.camera,
            texture_id: texture_id.to_owned(),
            rect,
            source: None,
            tint,
            flip_x,
            rotation,
        }));
    }

    fn draw_image_region(&mut self, texture_id: &str, rect: Rect, source: Rect, tint: Color) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            camera: self.camera,
            texture_id: texture_id.to_owned(),
            rect,
            source: Some(source),
            tint,
            flip_x: false,
            rotation: 0,
        }));
    }

    fn draw_image_region_transformed(
        &mut self,
        texture_id: &str,
        rect: Rect,
        source: Rect,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            camera: self.camera,
            texture_id: texture_id.to_owned(),
            rect,
            source: Some(source),
            tint,
            flip_x,
            rotation,
        }));
    }
}

fn to_wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color.r as f64,
        g: color.g as f64,
        b: color.b as f64,
        a: color.a as f64,
    }
}

fn transformed_rect_points(rect: Rect, rotation: i32) -> [Vec2; 4] {
    let center = Vec2::new(
        rect.origin.x + rect.size.x * 0.5,
        rect.origin.y + rect.size.y * 0.5,
    );
    let half = Vec2::new(rect.size.x * 0.5, rect.size.y * 0.5);
    let rotation = (rotation.rem_euclid(360) as f32).to_radians();
    let cos = rotation.cos();
    let sin = rotation.sin();
    let offsets = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ];

    offsets.map(|offset| {
        Vec2::new(
            center.x + offset.x * cos - offset.y * sin,
            center.y + offset.x * sin + offset.y * cos,
        )
    })
}

fn create_vertex_buffer(
    device: &wgpu::Device,
    label: &'static str,
    size: wgpu::BufferAddress,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn feature_summary(features: wgpu::Features) -> String {
    let known_features = [
        ("timestamp", wgpu::Features::TIMESTAMP_QUERY),
        ("bc", wgpu::Features::TEXTURE_COMPRESSION_BC),
        ("etc2", wgpu::Features::TEXTURE_COMPRESSION_ETC2),
        ("astc", wgpu::Features::TEXTURE_COMPRESSION_ASTC),
        ("f16", wgpu::Features::SHADER_F16),
        ("texture_arrays", wgpu::Features::TEXTURE_BINDING_ARRAY),
        (
            "storage_arrays",
            wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY,
        ),
        ("buffer_arrays", wgpu::Features::BUFFER_BINDING_ARRAY),
    ];
    let names = known_features
        .iter()
        .filter_map(|(name, feature)| features.contains(*feature).then_some(*name))
        .collect::<Vec<_>>();

    if names.is_empty() {
        "-".to_owned()
    } else {
        names.join(",")
    }
}

fn byte_len<T>(items: &[T]) -> wgpu::BufferAddress {
    std::mem::size_of_val(items) as wgpu::BufferAddress
}
