pub mod vertex;

use std::sync::Arc;

use wgpu_dance::{
    app::{WgpuApp, WgpuAppHandler},
    camera::{Camera, CameraBuddle},
    model::{Model, RenderVertex},
    texture::Texture,
};

use winit::{dpi::PhysicalSize, event::KeyEvent, event_loop::EventLoop, window::Window};

struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,

    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,

    render_pipeline: wgpu::RenderPipeline,

    model: Model<vertex::Vertex>,

    diffuse_bind_group: wgpu::BindGroup,

    camera: CameraBuddle,
}

impl WgpuApp for App {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None, // 追踪 API 调用路径
            )
            .await
            .unwrap();

        let size = window.inner_size();

        let caps = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture =
            Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let camera = Camera {
            // 将摄像机向上移动 1 个单位，向后移动 2 个单位
            // +z 朝向屏幕外
            eye: (0.0, 1.0, 2.0).into(),
            // 摄像机看向原点
            target: (0.0, 0.0, 0.0).into(),
            // 定义哪个方向朝上
            up: glam::Vec3::Y,
            aspect: surface_config.width as f32 / surface_config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let camera = CameraBuddle::new(camera, 0.2, &device);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera.bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("vs_main"),                     // 1.
                buffers: &[vertex::Vertex::buffer_layout_desc()], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // 将此设置为 Fill 以外的任何值都要需要开启 Feature::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // 需要开启 Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // 需要开启 Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,
        });

        let mut model = Model::new(vertex::VERTICES, vertex::INDICES, "simple model");
        model.alloc_buffer(&device);

        Self {
            device,
            queue,

            surface,
            surface_config,

            size,
            size_changed: false,

            render_pipeline,

            model,

            diffuse_bind_group,

            camera,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.resize_surface_if_needed();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.model.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.set_index_buffer(
                self.model.index_buffer.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint32,
            ); // 1.
            render_pass.draw_indexed(0..(self.model.indices.len() as u32), 0, 0..1);
            // 2.
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    /// 记录窗口大小已发生变化
    ///
    /// # NOTE:
    /// 当缩放浏览器窗口时, 窗口大小会以高于渲染帧率的频率发生变化，
    /// 如果窗口 size 发生变化就立即调整 surface 大小, 会导致缩放浏览器窗口大小时渲染画面闪烁。
    fn set_window_resized(&mut self, new_size: PhysicalSize<u32>) {
        if new_size == self.size {
            return;
        }
        self.size = new_size;
        self.size_changed = true;
    }

    /// 必要的时候调整 surface 大小
    fn resize_surface_if_needed(&mut self) {
        if self.size_changed {
            self.surface_config.width = self.size.width;
            self.surface_config.height = self.size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.size_changed = false;
        }
    }

    fn keyboard_input(&mut self, event: &KeyEvent) -> bool {
        self.camera.controller.process_events(event)
    }

    fn update(&mut self) {
        self.camera.update(&self.queue);
    }
}

fn main() -> Result<(), impl std::error::Error> {
    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuAppHandler::<App>::new("camera example");
    events_loop.run_app(&mut app)
}
