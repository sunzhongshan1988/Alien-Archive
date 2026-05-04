use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{Camera2d, Color, Rect, Renderer, Vec2};

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
    rect: Rect,
    color: Color,
}

#[derive(Clone)]
struct ImageCommand {
    texture_id: String,
    rect: Rect,
    source: Option<Rect>,
    tint: Color,
}

enum RenderCommand {
    Rect(RectCommand),
    Image(ImageCommand),
}

enum PreparedCommand {
    Rect {
        buffer: wgpu::Buffer,
        vertex_count: u32,
    },
    Image {
        texture_id: String,
        buffer: wgpu::Buffer,
        vertex_count: u32,
    },
}

struct GpuTexture {
    _texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    size: Vec2,
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
}

impl WgpuRenderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        // The window outlives the renderer inside the app event loop.
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(window)
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Alien Archive Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
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
            push_constant_ranges: &[],
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rectangle Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[RectVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
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
            multiview: None,
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
                bind_group_layouts: &[&image_bind_group_layout],
                push_constant_ranges: &[],
            });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Image Pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &image_shader,
                entry_point: "vs_main",
                buffers: &[ImageVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &image_shader,
                entry_point: "fs_main",
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
            multiview: None,
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
            textures: HashMap::new(),
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
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(wgpu::SurfaceError::OutOfMemory) => bail!("GPU surface is out of memory"),
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

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(to_wgpu_color(self.clear_color)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            for command in &prepared_commands {
                match command {
                    PreparedCommand::Rect {
                        buffer,
                        vertex_count,
                    } => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.set_vertex_buffer(0, buffer.slice(..));
                        pass.draw(0..*vertex_count, 0..1);
                    }
                    PreparedCommand::Image {
                        texture_id,
                        buffer,
                        vertex_count,
                    } => {
                        let Some(texture) = self.textures.get(texture_id) else {
                            continue;
                        };

                        pass.set_pipeline(&self.image_pipeline);
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.set_vertex_buffer(0, buffer.slice(..));
                        pass.draw(0..*vertex_count, 0..1);
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn prepare_commands(&self) -> Vec<PreparedCommand> {
        let mut prepared = Vec::with_capacity(self.commands.len());

        for command in &self.commands {
            match command {
                RenderCommand::Rect(command) => {
                    let vertices = self.build_rect_vertices(*command);
                    prepared.push(PreparedCommand::Rect {
                        buffer: self.create_vertex_buffer("Rectangle Vertex Buffer", &vertices),
                        vertex_count: vertices.len() as u32,
                    });
                }
                RenderCommand::Image(command) => {
                    if !self.textures.contains_key(&command.texture_id) {
                        continue;
                    }

                    let Some(texture) = self.textures.get(&command.texture_id) else {
                        continue;
                    };

                    let vertices = self.build_image_vertices(command, texture.size);
                    prepared.push(PreparedCommand::Image {
                        texture_id: command.texture_id.clone(),
                        buffer: self.create_vertex_buffer("Image Vertex Buffer", &vertices),
                        vertex_count: vertices.len() as u32,
                    });
                }
            }
        }

        prepared
    }

    fn create_vertex_buffer<T>(&self, label: &str, vertices: &[T]) -> wgpu::Buffer
    where
        T: Pod,
    {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
    }

    fn build_rect_vertices(&self, command: RectCommand) -> [RectVertex; 6] {
        let rect = command.rect;
        let color = [
            command.color.r,
            command.color.g,
            command.color.b,
            command.color.a,
        ];

        let top_left = self.world_to_clip(rect.origin);
        let top_right = self.world_to_clip(Vec2::new(rect.right(), rect.origin.y));
        let bottom_right = self.world_to_clip(Vec2::new(rect.right(), rect.bottom()));
        let bottom_left = self.world_to_clip(Vec2::new(rect.origin.x, rect.bottom()));

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
        let uv_left = source.origin.x / texture_size.x;
        let uv_top = source.origin.y / texture_size.y;
        let uv_right = source.right() / texture_size.x;
        let uv_bottom = source.bottom() / texture_size.y;

        let top_left = self.world_to_clip(rect.origin);
        let top_right = self.world_to_clip(Vec2::new(rect.right(), rect.origin.y));
        let bottom_right = self.world_to_clip(Vec2::new(rect.right(), rect.bottom()));
        let bottom_left = self.world_to_clip(Vec2::new(rect.origin.x, rect.bottom()));

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

    fn world_to_clip(&self, point: Vec2) -> [f32; 2] {
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let relative = (point - self.camera.position) * self.camera.zoom;
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
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
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
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
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

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.commands
            .push(RenderCommand::Rect(RectCommand { rect, color }));
    }

    fn draw_image(&mut self, texture_id: &str, rect: Rect, tint: Color) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            texture_id: texture_id.to_owned(),
            rect,
            source: None,
            tint,
        }));
    }

    fn draw_image_region(&mut self, texture_id: &str, rect: Rect, source: Rect, tint: Color) {
        self.commands.push(RenderCommand::Image(ImageCommand {
            texture_id: texture_id.to_owned(),
            rect,
            source: Some(source),
            tint,
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
