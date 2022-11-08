use crate::rendering_engine::model::vertex::*;
use crate::rendering_engine::shaders::*;
use crate::rendering_engine::{RenderStage, ViewProjection};
use nalgebra_glm::{ortho, perspective};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::AttachmentImage;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::{
    graphics::{
        input_assembly::InputAssemblyState, vertex_input::BuffersDefinition,
        viewport::ViewportState,
    },
    GraphicsPipeline, Pipeline,
};
use vulkano::render_pass::Subpass;
use vulkano::swapchain::PresentMode;
use vulkano::sync::GpuFuture;
use vulkano::{
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::{Swapchain, SwapchainCreateInfo},
    sync, VulkanLibrary,
};
use vulkano_win::VkSurfaceBuild;
use winit::window::Window;
use winit::{event_loop::EventLoop, window::WindowBuilder};

// Implementations of everything needed to initialize the rendering engine
impl super::RenderingEngine {
    pub fn init(event_loop: &EventLoop<()>) -> Self {
        // Instance creation
        let lib = VulkanLibrary::new()
            .unwrap_or_else(|err| panic!("failed to load Vulkan library: {:?}", err));
        let instance = {
            let enabled_extensions = vulkano_win::required_extensions(&lib);
            Instance::new(
                lib,
                InstanceCreateInfo {
                    enabled_extensions,
                    enumerate_portability: true,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|err| panic!("failed to create Vulkan instance: {:?}", err))
        };

        // Creates a surface on which to render
        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .expect("failed to create a surface to render on");

        // Device creation
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (p_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap_or_else(|err| panic!("failed to enumerate physical devices: {:?}", err))
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.graphics
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("could not find a satisfying physical device");

        println!(
            "using device {} (type: {:?})",
            p_device.properties().device_name,
            p_device.properties().device_type
        );

        let (device, mut queues) = Device::new(
            p_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap_or_else(|err| panic!("could not create device: {:?}", err));
        let queue = queues.next().unwrap();

        // Swapchain creation
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };
        let (swapchain, images) = {
            let capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap_or_else(|err| {
                    panic!("unable to determine surface capabilities: {:?}", err)
                });
            let image_format = Some(
                device
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );
            let present_mode = p_device
                .surface_present_modes(&surface)
                .unwrap()
                .min_by_key(|m| match m {
                    PresentMode::Immediate => 0,
                    PresentMode::Mailbox => 1,
                    PresentMode::Fifo => 2,
                    _ => 3,
                })
                .unwrap();

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: capabilities.min_image_count,
                    image_format,
                    image_extent: surface.window().inner_size().into(),
                    image_usage: ImageUsage {
                        color_attachment: true,
                        ..ImageUsage::empty()
                    },
                    composite_alpha: capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    present_mode,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|err| panic!("failed to create a swapchain: {:?}", err))
        };

        // Shaders compilation
        let deferred_vertex_shader = deferred_vertex_shader::load(device.clone())
            .unwrap_or_else(|err| panic!("failed to compile deferred vertex shader: {:?}", err));
        let deferred_fragment_shader = deferred_fragment_shader::load(device.clone())
            .unwrap_or_else(|err| panic!("failed to compile deferred fragment shader: {:?}", err));
        let ambient_vertex_shader = ambient_vertex_shader::load(device.clone())
            .unwrap_or_else(|err| panic!("failed to compile ambient vertex shader: {:?}", err));
        let ambient_fragment_shader = ambient_fragment_shader::load(device.clone())
            .unwrap_or_else(|err| panic!("failed to compile ambient fragment shader: {:?}", err));
        let directional_vertex_shader = directional_vertex_shader::load(device.clone())
            .unwrap_or_else(|err| panic!("failed to compile directional vertex shader: {:?}", err));
        let directional_fragment_shader = directional_fragment_shader::load(device.clone())
            .unwrap_or_else(|err| {
                panic!("failed to compile directional fragment shader: {:?}", err)
            });

        // Renderpass creation
        // We use a two-step render pass to get more flexibility
        let render_pass = vulkano::ordered_passes_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                vertex_color: {
                    load: Clear,
                    store: DontCare,
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    samples: 1,
                },
                normal: {
                    load: Clear,
                    store: DontCare,
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 1,
                },
                frag_pos: {
                    load: Clear,
                    store: DontCare,
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 1,
                },
                specular: {
                    load: Clear,
                    store: DontCare,
                    format: Format::R16G16_SFLOAT,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D32_SFLOAT_S8_UINT,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [vertex_color, normal, frag_pos, specular], // Color is to be understood as "output" here
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [color],
                    depth_stencil: {},
                    input: [vertex_color, normal, frag_pos, specular]
                }
            ]
        )
        .unwrap_or_else(|err| panic!("failed to create a render pass: {:?}", err));

        // Graphics pipeline creation
        let deferred_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(deferred_vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| panic!("failed to build deferred render pass: {:?}", err));

        let ambient_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(ambient_vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(ambient_fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(
                    Subpass::from(render_pass.clone(), 1)
                        .unwrap()
                        .num_color_attachments(),
                )
                .blend(AttachmentBlend {
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::One,
                    color_destination: BlendFactor::One,
                    alpha_op: BlendOp::Max,
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                }),
            )
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| {
                panic!("failed to build ambient lighting render pass: {:?}", err)
            });

        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(directional_vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(
                    Subpass::from(render_pass.clone(), 1)
                        .unwrap()
                        .num_color_attachments(),
                )
                .blend(AttachmentBlend {
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::One,
                    color_destination: BlendFactor::One,
                    alpha_op: BlendOp::Max,
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                }),
            )
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| {
                panic!(
                    "failed to build directional lighting render pass: {:?}",
                    err
                )
            });

        // Framebuffer creation
        let (framebuffers, vertex_color_buffer, normal_buffer, frag_pos_buffer, specular_buffer) = window_size_dependent_setup(
            device.clone(),
            &images,
            render_pass.clone(),
            &mut viewport,
        );

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        // Buffers and pools
        let viewport_span_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            },
            false,
            DummyVertex::cover_viewport().iter().cloned(),
        )
        .unwrap();

        let mut vp_matrix = ViewProjection::default();
        let dimensions: [f32; 2] = surface.window().inner_size().into();
        vp_matrix.projection = perspective(dimensions[0] / dimensions[1], 90.0, 0.01, 100.0);
        let vp_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            deferred_vertex_shader::ty::VP {
                view: vp_matrix.view.into(),
                projection: vp_matrix.projection.into(),
            },
        )
        .unwrap();
        let vp_layout = deferred_pipeline.layout().set_layouts().get(0).unwrap();
        let vp_descriptor_set = PersistentDescriptorSet::new(
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, vp_buffer.clone())],
        )
        .unwrap();

        let ambient_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            ambient_fragment_shader::ty::AmbientLight {
                color: [1f32; 3],
                intensity: 0.2,
            },
        )
        .unwrap();

        let model_buffer_pool =
            CpuBufferPool::<deferred_vertex_shader::ty::Model>::uniform_buffer(device.clone());
        let material_buffer_pool =
            CpuBufferPool::<deferred_fragment_shader::ty::Material>::uniform_buffer(device.clone());
        let directional_buffer_pool = CpuBufferPool::<
            directional_fragment_shader::ty::LightSource,
        >::uniform_buffer(device.clone());

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            swapchain,
            render_pass,
            framebuffers,

            frag_pos_buffer,
            vertex_color_buffer,
            specular_buffer,
            normal_buffer,
            viewport_span_buffer,
            vp_buffer,
            vp_descriptor_set,
            model_buffer_pool,
            material_buffer_pool,
            directional_buffer_pool,
            ambient_buffer,

            deferred_pipeline,
            ambient_pipeline,
            directional_pipeline,

            vp_matrix,
            viewport,

            bound_model_handle: None,
            previous_frame_end,
            render_stage: RenderStage::Stopped,
            commands: None,
            image_index: 0,
            future_handle: None,
        }
    }
}

type FramebuffersAndAttachments = (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
);
pub fn window_size_dependent_setup(
    device: Arc<Device>,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> FramebuffersAndAttachments {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];
    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(
            device.clone(),
            dimensions,
            Format::D32_SFLOAT_S8_UINT
        ).unwrap(),
    ).unwrap();

    let vertex_color_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::A2B10G10R10_UNORM_PACK32,
        ).unwrap(),
    ).unwrap();

    let normal_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::R16G16B16A16_SFLOAT,
        ).unwrap(),
    ).unwrap();

    let frag_pos_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::R16G16B16A16_SFLOAT
        ).unwrap()
    ).unwrap();
    let specular_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::R16G16_SFLOAT
        ).unwrap()
    ).unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        vertex_color_buffer.clone(),
                        normal_buffer.clone(),
                        frag_pos_buffer.clone(),
                        specular_buffer.clone(),
                        depth_buffer.clone(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // We also need to return our attachments' buffers
    (framebuffers, vertex_color_buffer, normal_buffer, frag_pos_buffer, specular_buffer)
}
