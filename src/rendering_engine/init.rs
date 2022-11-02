use crate::rendering_engine::shaders::*;
use crate::rendering_engine::model::vertex::Vertex;
use std::sync::Arc;
use vulkano::pipeline::{
    graphics::{
        input_assembly::InputAssemblyState, vertex_input::BuffersDefinition,
        viewport::ViewportState,
    },
    GraphicsPipeline,
};
use vulkano::render_pass::Subpass;
use vulkano::sync::GpuFuture;
use vulkano::{
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features,
        QueueCreateInfo,
    },
    image::{view::ImageView, ImageAccess, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::{Swapchain, SwapchainCreateInfo},
    sync, Version, VulkanLibrary,
    format::Format
};
use vulkano::image::AttachmentImage;
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
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
            let extensions = vulkano_win::required_extensions(&lib);
            Instance::new(
                lib,
                InstanceCreateInfo {
                    enabled_extensions: extensions,
                    max_api_version: Some(Version::V1_1),
                    ..Default::default()
                },
            )
            .unwrap_or_else(|err| panic!("failed to create Vulkan instance: {:?}", err))
        };

        // Creates a surface on which to render
        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, instance.clone())
            .expect("failed to create a surface to render on");

        // Device creation
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (p_device, queue_family) = instance
            .enumerate_physical_devices()
            .unwrap_or_else(|err| panic!("failed to enumerate physical devices: {:?}", err))
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .find(|&q| q.queue_flags.graphics)
                    .map(|q| (p.clone(), q.clone()))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("could not find a satisfying physical device");
        let (device, mut queues) = Device::new(
            p_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: Features::empty(),
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index: 0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap_or_else(|err| panic!("could not create device: {:?}", err));

        // Swapchain creation
        let (mut swapchain, images) = {
            let capabilities = p_device
                .clone()
                .surface_capabilities(&surface, Default::default())
                .unwrap_or_else(|err| {
                    panic!("unable to determine surface capabilities: {:?}", err)
                });
            let image_usage = capabilities.supported_usage_flags;
            let composite_alpha = capabilities
                .supported_composite_alpha
                .iter()
                .next()
                .unwrap();
            let image_format = Some(
                p_device
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: capabilities.min_image_count,
                    image_format,
                    image_extent: surface.window().inner_size().into(),
                    image_usage,
                    composite_alpha,
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
            .unwrap_or_else(|err| panic!("failed to compile directional fragment shader: {:?}", err));

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
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [vertex_color, normal], // Color is to be understood as "output" here
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [color],
                    depth_stencil: {},
                    input: [vertex_color, normal]
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
            .rasterization_state(RasterizationState::new()
                .cull_mode(CullMode::Back)
            )
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| panic!("failed to build deferred render pass: {:?}", err));

        let ambient_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(ambient_vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(ambient_fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(Subpass::from(render_pass.clone(), 1).unwrap().num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new()
                .cull_mode(CullMode::Back)
            )
            .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| panic!("failed to build lighting render pass: {:?}", err));

        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(directional_vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(Subpass::from(render_pass.clone(), 1).unwrap().num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new()
                .cull_mode(CullMode::Back)
            )
            .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
            .build(device.clone())
            .unwrap_or_else(|err| panic!("failed to build lighting render pass: {:?}", err));

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        // Framebuffer creation
        let (framebuffers, vertex_color_buffer, normal_buffer) =
            window_size_dependent_setup(device.clone(), &images, render_pass.clone(), &mut viewport);

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        Self {
            _instance: instance,
            surface,
            device,
            queue: queues.next().unwrap(),
            swapchain,
            render_pass,
            framebuffers,
            vertex_color_buffer,
            normal_buffer,
            deferred_pipeline,
            ambient_pipeline,
            directional_pipeline,
            viewport,

            deferred_vertex_shader,
            deferred_fragment_shader,
            ambient_vertex_shader,
            ambient_fragment_shader,
            directional_vertex_shader,
            directional_fragment_shader,

            previous_frame_end,
            invalid_swapchain: false,
        }
    }
}

pub fn window_size_dependent_setup(
    device: Arc<Device>,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>
) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];
    let vertex_color_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::A2B10G10R10_UNORM_PACK32
        ).unwrap()
    ).unwrap();
    let normal_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            device.clone(),
            dimensions,
            Format::R16G16B16A16_SFLOAT
        ).unwrap()
    ).unwrap();
    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(
            device.clone(),
            dimensions,
            Format::D16_UNORM
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
                        depth_buffer.clone()
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // We also need to return our attachments' buffers
    (framebuffers, vertex_color_buffer.clone(), normal_buffer.clone())
}
