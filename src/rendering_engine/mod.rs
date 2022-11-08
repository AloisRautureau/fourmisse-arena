mod init;
mod lights;
mod maintenance_operations;
mod materials;
mod model;
mod render;
mod resource_handler;
mod shaders;
mod view_projection;

use std::convert::Into;
pub use lights::*;
pub use materials::Material;
pub use model::vertex::*;
pub use model::Model;
pub use resource_handler::{ResourceHandle, ResourceHandler, ResourceVec};
pub use shaders::*;
pub use view_projection::ViewProjection;

use nalgebra_glm::TVec3;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::pool::standard::{
    StandardCommandPoolAlloc, StandardCommandPoolBuilder,
};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::view::ImageView;
use vulkano::image::AttachmentImage;
use vulkano::pipeline::Pipeline;
use vulkano::swapchain::SwapchainAcquireFuture;
use vulkano::{
    device::{Device, Queue},
    instance::Instance,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{Framebuffer, RenderPass},
    swapchain::{Surface, Swapchain},
    sync::GpuFuture,
};
use winit::window::Window;

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum RenderStage {
    Stopped,
    Deferred,
    Directional,
    Ambient,
    Invalid, // Case where redrawing the frame is necessary
}

pub struct RenderingEngine {
    // Necessary constructs
    _instance: Arc<Instance>,
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,

    // Buffers / pools and descriptor sets
    frag_pos_buffer: Arc<ImageView<AttachmentImage>>,
    specular_buffer: Arc<ImageView<AttachmentImage>>,
    vertex_color_buffer: Arc<ImageView<AttachmentImage>>,
    normal_buffer: Arc<ImageView<AttachmentImage>>,
    viewport_span_buffer: Arc<CpuAccessibleBuffer<[DummyVertex]>>,
    vp_buffer: Arc<CpuAccessibleBuffer<deferred_vertex_shader::ty::VP>>,
    vp_descriptor_set: Arc<PersistentDescriptorSet>,
    model_buffer_pool: CpuBufferPool<deferred_vertex_shader::ty::Model>,
    material_buffer_pool: CpuBufferPool<deferred_fragment_shader::ty::Material>,
    directional_buffer_pool: CpuBufferPool<directional_fragment_shader::ty::LightSource>,
    ambient_buffer: Arc<CpuAccessibleBuffer<ambient_fragment_shader::ty::AmbientLight>>,

    // Pipelines
    deferred_pipeline: Arc<GraphicsPipeline>,
    ambient_pipeline: Arc<GraphicsPipeline>,
    directional_pipeline: Arc<GraphicsPipeline>,

    // Swapchain related constructs
    framebuffers: Vec<Arc<Framebuffer>>,
    swapchain: Arc<Swapchain<Window>>,
    vp_matrix: ViewProjection,
    viewport: Viewport,

    // Variables used during rendering
    bound_model_handle: Option<ResourceHandle>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub render_stage: RenderStage,
    commands: Option<
        AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<StandardCommandPoolAlloc>,
            StandardCommandPoolBuilder,
        >,
    >,
    image_index: usize,
    future_handle: Option<SwapchainAcquireFuture<Window>>,
}
impl RenderingEngine {
    pub fn move_camera(&mut self, delta: &TVec3<f32>) {
        self.vp_matrix.move_camera(delta);
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            deferred_vertex_shader::ty::VP {
                view: self.vp_matrix.view.into(),
                projection: self.vp_matrix.projection.into(),
            },
        )
        .unwrap();

        let vp_layout = self
            .deferred_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        self.vp_descriptor_set = PersistentDescriptorSet::new(
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())],
        )
        .unwrap();

        self.render_stage = RenderStage::Stopped;
    }

    pub fn set_ambient_light_source(&mut self, ambient_light: &AmbientLightSource) {
        self.ambient_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            ambient_fragment_shader::ty::AmbientLight {
                color: ambient_light.color,
                intensity: ambient_light.intensity,
            },
        )
        .unwrap()

        // Changing the ambient light source during the rendering of a frame
        // has no consequence, so we do not invalidate the currently drawn frame
    }
}
