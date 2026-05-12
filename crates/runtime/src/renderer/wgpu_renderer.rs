use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{Camera2d, Color, Rect, RenderStats, Renderer, Vec2};

const MIN_VERTEX_BUFFER_BYTES: u64 = 4096;
type TextureIndex = usize;

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

#[derive(Clone, Copy)]
struct ImageCommand {
    camera: Camera2d,
    texture_index: Option<TextureIndex>,
    is_ground_chunk: bool,
    rect: Rect,
    source: Option<Rect>,
    tint: Color,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Copy)]
enum RenderCommand {
    Rect(RectCommand),
    Image(ImageCommand),
}

#[derive(Clone, Copy)]
enum PreparedCommand {
    Rect {
        buffer_index: usize,
        vertex_count: u32,
    },
    Image {
        texture_index: TextureIndex,
        buffer_index: usize,
        vertex_count: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingBatchKind {
    Rect,
    Image(TextureIndex),
}

struct GpuTexture {
    _texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    size: Vec2,
}

struct ReusableVertexBuffer {
    buffer: wgpu::Buffer,
    capacity_bytes: u64,
}

#[derive(Default)]
struct VertexBufferPool {
    buffers: Vec<ReusableVertexBuffer>,
    next: usize,
}

impl VertexBufferPool {
    fn reset(&mut self) {
        self.next = 0;
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
    prepared_commands: Vec<PreparedCommand>,
    texture_lookup: HashMap<String, TextureIndex>,
    textures: Vec<GpuTexture>,
    last_frame_stats: RenderStats,
    rect_vertex_buffers: VertexBufferPool,
    image_vertex_buffers: VertexBufferPool,
    pending_rect_vertices: Vec<RectVertex>,
    pending_image_vertices: Vec<ImageVertex>,
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

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Alien Archive Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .context("failed to create GPU device")?;

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
            prepared_commands: Vec::with_capacity(256),
            texture_lookup: HashMap::new(),
            textures: Vec::new(),
            last_frame_stats: RenderStats::default(),
            rect_vertex_buffers: VertexBufferPool::default(),
            image_vertex_buffers: VertexBufferPool::default(),
            pending_rect_vertices: Vec::with_capacity(256),
            pending_image_vertices: Vec::with_capacity(256),
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
        self.prepare_commands();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
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
                timestamp_writes: None,
                multiview_mask: None,
            });

            for command_index in 0..self.prepared_commands.len() {
                let command = self.prepared_commands[command_index];
                match command {
                    PreparedCommand::Rect {
                        buffer_index,
                        vertex_count,
                    } => {
                        let Some(buffer) = self.rect_vertex_buffers.buffers.get(buffer_index)
                        else {
                            continue;
                        };

                        pass.set_pipeline(&self.rect_pipeline);
                        pass.set_vertex_buffer(0, buffer.buffer.slice(..));
                        pass.draw(0..vertex_count, 0..1);
                    }
                    PreparedCommand::Image {
                        texture_index,
                        buffer_index,
                        vertex_count,
                    } => {
                        let Some(texture) = self.textures.get(texture_index) else {
                            continue;
                        };
                        let Some(buffer) = self.image_vertex_buffers.buffers.get(buffer_index)
                        else {
                            continue;
                        };

                        pass.set_pipeline(&self.image_pipeline);
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.set_vertex_buffer(0, buffer.buffer.slice(..));
                        pass.draw(0..vertex_count, 0..1);
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn prepare_commands(&mut self) {
        self.rect_vertex_buffers.reset();
        self.image_vertex_buffers.reset();
        self.pending_rect_vertices.clear();
        self.pending_image_vertices.clear();
        self.prepared_commands.clear();

        let mut pending: Option<PendingBatchKind> = None;
        let mut stats = RenderStats {
            queued_commands: self.commands.len(),
            loaded_textures: self.textures.len(),
            ..RenderStats::default()
        };

        for command_index in 0..self.commands.len() {
            let command = self.commands[command_index];
            match command {
                RenderCommand::Rect(command) => {
                    stats.rect_commands += 1;
                    let vertices = self.build_rect_vertices(command);
                    if pending != Some(PendingBatchKind::Rect) {
                        self.flush_pending_batch(&mut pending, &mut stats);
                        pending = Some(PendingBatchKind::Rect);
                    }
                    self.pending_rect_vertices.extend_from_slice(&vertices);
                }
                RenderCommand::Image(command) => {
                    stats.image_commands += 1;
                    if command.is_ground_chunk {
                        stats.ground_chunk_commands += 1;
                    }

                    let Some(texture_index) = command.texture_index else {
                        stats.skipped_image_commands += 1;
                        continue;
                    };

                    let Some(texture) = self.textures.get(texture_index) else {
                        stats.skipped_image_commands += 1;
                        continue;
                    };

                    let vertices = self.build_image_vertices(&command, texture.size);
                    if pending != Some(PendingBatchKind::Image(texture_index)) {
                        self.flush_pending_batch(&mut pending, &mut stats);
                        pending = Some(PendingBatchKind::Image(texture_index));
                    }
                    self.pending_image_vertices.extend_from_slice(&vertices);
                }
            }
        }

        self.flush_pending_batch(&mut pending, &mut stats);
        stats.draw_calls = self.prepared_commands.len();
        self.last_frame_stats = stats;
    }

    fn flush_pending_batch(
        &mut self,
        pending: &mut Option<PendingBatchKind>,
        stats: &mut RenderStats,
    ) {
        let Some(batch_kind) = pending.take() else {
            return;
        };

        match batch_kind {
            PendingBatchKind::Rect => {
                if self.pending_rect_vertices.is_empty() {
                    return;
                }
                stats.rect_batches += 1;
                stats.vertex_buffers += 1;
                let vertex_count = self.pending_rect_vertices.len() as u32;
                let buffer_index = Self::write_vertices_to_pool(
                    &self.device,
                    &self.queue,
                    &mut self.rect_vertex_buffers,
                    "Rectangle Vertex Buffer",
                    &self.pending_rect_vertices,
                );
                self.prepared_commands.push(PreparedCommand::Rect {
                    buffer_index,
                    vertex_count,
                });
                self.pending_rect_vertices.clear();
            }
            PendingBatchKind::Image(texture_index) => {
                if self.pending_image_vertices.is_empty() {
                    return;
                }
                stats.image_batches += 1;
                stats.vertex_buffers += 1;
                let vertex_count = self.pending_image_vertices.len() as u32;
                let buffer_index = Self::write_vertices_to_pool(
                    &self.device,
                    &self.queue,
                    &mut self.image_vertex_buffers,
                    "Image Vertex Buffer",
                    &self.pending_image_vertices,
                );
                self.prepared_commands.push(PreparedCommand::Image {
                    texture_index,
                    buffer_index,
                    vertex_count,
                });
                self.pending_image_vertices.clear();
            }
        }
    }

    fn write_vertices_to_pool<T>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pool: &mut VertexBufferPool,
        label: &str,
        vertices: &[T],
    ) -> usize
    where
        T: Pod,
    {
        let bytes = bytemuck::cast_slice(vertices);
        let required_bytes = bytes.len() as u64;
        let capacity_bytes = required_bytes
            .max(MIN_VERTEX_BUFFER_BYTES)
            .next_power_of_two();
        let buffer_index = pool.next;

        if buffer_index == pool.buffers.len() {
            pool.buffers.push(ReusableVertexBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: capacity_bytes,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                capacity_bytes,
            });
        } else if pool.buffers[buffer_index].capacity_bytes < required_bytes {
            pool.buffers[buffer_index] = ReusableVertexBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: capacity_bytes,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                capacity_bytes,
            };
        }

        queue.write_buffer(&pool.buffers[buffer_index].buffer, 0, bytes);
        pool.next += 1;
        buffer_index
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

        let texture = GpuTexture {
            _texture: texture,
            _sampler: sampler,
            bind_group,
            size: Vec2::new(width as f32, height as f32),
        };
        if let Some(index) = self.texture_lookup.get(id).copied() {
            if let Some(slot) = self.textures.get_mut(index) {
                *slot = texture;
            }
        } else {
            let index = self.textures.len();
            self.textures.push(texture);
            self.texture_lookup.insert(id.to_owned(), index);
        }

        Ok(())
    }

    fn texture_size(&self, id: &str) -> Option<Vec2> {
        self.texture_lookup
            .get(id)
            .and_then(|index| self.textures.get(*index))
            .map(|texture| texture.size)
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(self.config.width as f32, self.config.height as f32)
    }

    fn frame_stats(&self) -> RenderStats {
        self.last_frame_stats
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
        self.queue_image_command(texture_id, rect, None, tint, false, 0);
    }

    fn draw_image_transformed(
        &mut self,
        texture_id: &str,
        rect: Rect,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        self.queue_image_command(texture_id, rect, None, tint, flip_x, rotation);
    }

    fn draw_image_region(&mut self, texture_id: &str, rect: Rect, source: Rect, tint: Color) {
        self.queue_image_command(texture_id, rect, Some(source), tint, false, 0);
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
        self.queue_image_command(texture_id, rect, Some(source), tint, flip_x, rotation);
    }
}

impl WgpuRenderer {
    fn queue_image_command(
        &mut self,
        texture_id: &str,
        rect: Rect,
        source: Option<Rect>,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            camera: self.camera,
            texture_index: self.texture_lookup.get(texture_id).copied(),
            is_ground_chunk: texture_id.starts_with("__map_ground_cache_"),
            rect,
            source,
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
