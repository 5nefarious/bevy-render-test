use bevy::{
    prelude::*,
    window::{WindowId, WindowResized},
};
use std::{
    borrow::Cow,
    collections::HashSet,
};

pub struct Renderer {
    pub window_id: WindowId,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swapchain_desc: wgpu::SwapChainDescriptor,
    swapchain: wgpu::SwapChain,
    compute_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    render_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        window: &Window,
        winit_window: &winit::window::Window,
    ) -> Self {
        let window_id = window.id();
        let surface = unsafe { instance.create_surface(winit_window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
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

        let swapchain_format = adapter
            .get_swap_chain_preferred_format(&surface)
            .expect("Failed to get preferred swapchain format");
        
        let (width, height) = (window.physical_width(), window.physical_height());
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width,
            height,
            present_mode: if window.vsync() {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Immediate
            }
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

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
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::STORAGE,
            label: None,
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
            label: None,
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Vulkan | wgpu::Backend::Metal => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
            }
            _ => {}
        }
        let cs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/compute.wgsl"))),
            flags,
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shaders = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/draw.wgsl"))),
            flags,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shaders,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Renderer {
            window_id,
            surface,
            device,
            queue,
            swapchain_desc: sc_desc,
            swapchain,
            compute_bind_group,
            compute_pipeline,
            render_bind_group,
            render_pipeline,
        }
    }

    pub fn handle_resize(&mut self, window: &Window) {
        self.swapchain_desc.width = window.physical_width();
        self.swapchain_desc.height = window.physical_height();
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.swapchain_desc);
    }

    pub fn update(&self) {
        let sc_desc = &self.swapchain_desc;
        let mut encoder = 
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch(sc_desc.width / 32 + 1, sc_desc.height / 32 + 1, 1);
        }

        let frame = &self.swapchain
            .get_current_frame()
            .expect("Failed to get next view in swapchain")
            .output;
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
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