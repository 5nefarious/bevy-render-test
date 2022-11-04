use bevy::{
    prelude::*,
    window::{WindowId, WindowResized},
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use std::{
    collections::HashSet,
    fmt::Debug,
    time::Instant,
};

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RaySamplingUniform {
    seed: u32,
    pad: u32,   // Whyyyyyyyy?
    extent: [u32; 2],
}

pub struct RenderPipeline {
    compute: wgpu::ComputePipeline,
    render: wgpu::RenderPipeline,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    sampling_uniform: RaySamplingUniform,
    sampling_buffer: wgpu::Buffer,
    start: Instant,
}

impl RenderPipeline {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32
    ) -> Self {
        let start = Instant::now();

        let raybuffer = RenderPipeline::create_raybuffer(device, width, height);

        let sampling_uniform = RaySamplingUniform {
            seed: start.elapsed().subsec_nanos(),
            pad: 0,
            extent: [width, height],
        };

        let sampling_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray Sampling Buffer"),
            contents: bytemuck::cast_slice(&[sampling_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = RenderPipeline::create_compute_bindgroup(
            device,
            &compute_bind_group_layout,
            &raybuffer,
            &sampling_buffer,
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/compute.wgsl").into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &cs_module,
            entry_point: "main",
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let render_bind_group = RenderPipeline::create_render_bindgroup(
            device,
            &render_bind_group_layout,
            &raybuffer,
            &sampler,
        );
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shaders = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Presentation Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/draw.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shaders,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        RenderPipeline {
            compute: compute_pipeline,
            render: render_pipeline,
            compute_bind_group,
            render_bind_group,
            sampler,
            sampling_uniform,
            sampling_buffer,
            start,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        let extent = &self.sampling_uniform.extent;
        if extent[0] != width || extent[1] != height {
            let raybuffer = RenderPipeline::create_raybuffer(device, width, height);

            self.compute_bind_group = RenderPipeline::create_compute_bindgroup(
                device,
                &self.compute.get_bind_group_layout(0),
                &raybuffer,
                &self.sampling_buffer,
            );

            self.render_bind_group = RenderPipeline::create_render_bindgroup(
                device,
                &self.render.get_bind_group_layout(0),
                &raybuffer,
                &self.sampler,
            );

            self.sampling_uniform.extent = [width, height];
        }
        self.sampling_uniform.seed = self.start.elapsed().subsec_nanos();
        queue.write_buffer(&self.sampling_buffer, 0, bytemuck::cast_slice(&[self.sampling_uniform]));
    }

    fn create_raybuffer(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Compute Framebuffer"),
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_compute_bindgroup(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        raybuffer: &wgpu::TextureView,
        sampling_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&raybuffer),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sampling_buffer.as_entire_binding(),
                },
            ],
            label: None,
        })
    }

    fn create_render_bindgroup(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        raybuffer: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&raybuffer),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        })
    }
}

#[derive(Component)]
pub struct Renderer {
    pub window_id: WindowId,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: RenderPipeline,
}

impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        window: &Window,
    ) -> Self {
        let window_id = window.id();
        let surface = unsafe {
            let handle = window.raw_window_handle().get_handle();
            instance.create_surface(&handle) 
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        fn list_options<T: Debug>(opts: &Vec<T>) {
            for (i, fmt) in opts.iter().enumerate() {
                let bullet = if i > 0 {'-'} else {'*'};
                eprintln!(" {bullet} {fmt:?}");
            }
        }
        
        let surface_formats = surface.get_supported_formats(&adapter);
        assert!(surface_formats.len() > 0, "Surface format not supported by adapter");
        eprintln!("\nSurface Formats:");
        list_options(&surface_formats);

        let present_modes = surface.get_supported_modes(&adapter);
        assert!(present_modes.len() > 0, "Surface presentation mode not supported by adapter");
        eprintln!("\nPresent Modes:");
        list_options(&present_modes);

        // let alpha_modes = surface.get_supported_alpha_modes(&adapter);
        // assert!(alpha_modes.len() > 0, "Surface alpha mode not supported by adapter");
        // eprintln!("\nAlpha Modes:");
        // for mode in &alpha_modes {
        //     eprintln!(" - {mode:?}")
        // }
        eprintln!();
        
        let (width, height) = (window.physical_width(), window.physical_height());
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_formats[0],
            width,
            height,
            present_mode: present_modes[0],
            // alpha_mode: alpha_modes[0],
        };

        surface.configure(&device, &config);

        let pipeline = RenderPipeline::new(&device, &config, width, height);

        Renderer {
            window_id,
            surface,
            device,
            queue,
            config,
            pipeline,
        }
    }

    pub fn handle_resize(&mut self, window: &Window) {
        let width  = window.physical_width();
        let height = window.physical_height();
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn update(&mut self) {
        let config = &self.config;
        let pipeline = &mut self.pipeline;
        pipeline.update(&self.device, &self.queue, self.config.width, self.config.height);
        let mut encoder = 
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });
            cpass.set_pipeline(&pipeline.compute);
            cpass.set_bind_group(0, &pipeline.compute_bind_group, &[]);
            cpass.dispatch_workgroups(config.width / 16 + 1, config.height / 16 + 1, 1);
        }

        let output = self.surface.get_current_texture()
            .expect("Failed to get surface texture");
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&pipeline.render);
            rpass.set_bind_group(0, &pipeline.render_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

#[derive(Default)]
pub struct RenderSystemState {
    resized_window_ids: HashSet<WindowId>,
}

pub fn render_system(
    mut state: Local<RenderSystemState>,
    mut window_resized_events: EventReader<WindowResized>,
    windows: Res<Windows>,
    mut query: Query<&mut Renderer>,
) {
    for event in window_resized_events.iter() {
        state.resized_window_ids.insert(event.id);
    }

    for mut renderer in query.iter_mut() {
        let wid = renderer.window_id;
        if state.resized_window_ids.take(&wid).is_some() {
            let window = windows
                .get(wid)
                .expect("Received resize event from nonexistent window");
            renderer.handle_resize(window);
        }

        renderer.update();
    }
}