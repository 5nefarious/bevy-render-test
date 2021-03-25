use bevy::{
    prelude::*,
    window::{WindowId, WindowResized},
};
use image::{GenericImageView, io::Reader as ImageReader};
use std::{
    borrow::Cow,
    collections::HashSet,
    io::Cursor,
};

pub struct Renderer {
    pub window_id: WindowId,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swapchain_desc: wgpu::SwapChainDescriptor,
    swapchain: wgpu::SwapChain,
    #[allow(dead_code)]
    shaders: wgpu::ShaderModule,
    bind_group: wgpu::BindGroup,
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

        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface);
        
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window.physical_width(),
            height: window.physical_height(),
            present_mode: if window.vsync() {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Immediate
            }
        };

        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Vulkan | wgpu::Backend::Metal => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
            }
            _ => {}
        }
        let shaders = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders.wgsl"))),
            flags,
        });

        let image = ImageReader::new(Cursor::new(include_bytes!("logo.png")))
            .with_guessed_format()
            .expect("Failed to read image")
            .decode()
            .expect("Failed to decode image")
            .flipv();
        let (width, height) = image.dimensions();
        let texels = image.as_bytes();
        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_DST,
            label: None,
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            texels,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * width,
                rows_per_image: 0,
            },
            texture_extent,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        // let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        //     label: None,
        //     layout: None,
        //     module: &shaders,
        //     entry_point: "cs_main",
        // });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
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
            shaders,
            bind_group,
            render_pipeline,
        }
    }

    pub fn handle_resize(&mut self, window: &Window) {
        self.swapchain_desc.width = window.physical_width();
        self.swapchain_desc.height = window.physical_height();
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.swapchain_desc);
    }

    pub fn update(&self) {
        // let mut encoder = 
        //     device.create_command_encoder(&wgpu::CommandEncoderDescriptor {label: None });
        // {
        //     let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, });
        //     cpass.set_pipeline(&self.compute_pipeline);
        //     cpass.dispatch(1, 1, 1);
        // }

        let frame = &self.swapchain
            .get_current_frame()
            .expect("Failed to get next view in swapchain")
            .output;
        let mut encoder = 
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}

pub fn render_system(
    mut window_resized_events: EventReader<WindowResized>,
    windows: Res<Windows>,
    mut query: Query<&mut Renderer>,
) {
    let mut resized_window_ids = HashSet::new();
    for event in window_resized_events.iter() {
        resized_window_ids.insert(event.id);
    }

    for mut renderer in query.iter_mut() {
        let wid = renderer.window_id;
        if resized_window_ids.contains(&wid) {
            let window = windows
                .get(wid)
                .expect("Received resize event from nonexistent window");
            renderer.handle_resize(window);
        }

        renderer.update();
    }
}