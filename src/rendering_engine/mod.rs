mod init;
mod maintenance_operations;
mod shaders;
mod mvp;
mod lights;
mod model;

pub use model::Model;
pub use model::vertex::Vertex;
pub use shaders::*;
pub use mvp::ModelViewProjection;
pub use lights::*;

use std::sync::Arc;
use vulkano::{
    device::{Device, Queue},
    instance::Instance,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{Framebuffer, RenderPass},
    shader::ShaderModule,
    swapchain::{Surface, Swapchain},
    sync::GpuFuture,
};
use vulkano::image::AttachmentImage;
use vulkano::image::view::ImageView;
use winit::window::Window;

pub const CLEAR_COLOR: [f32; 4] = [0.1568, 0.1568, 0.1568, 1.0];

pub struct RenderingEngine {
    // Necessary constructs
    _instance: Arc<Instance>,
    pub surface: Arc<Surface<Window>>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub swapchain: Arc<Swapchain<Window>>,
    pub render_pass: Arc<RenderPass>,

    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub vertex_color_buffer: Arc<ImageView<AttachmentImage>>,
    pub normal_buffer: Arc<ImageView<AttachmentImage>>,

    pub deferred_pipeline: Arc<GraphicsPipeline>,
    pub ambient_pipeline: Arc<GraphicsPipeline>,
    pub directional_pipeline: Arc<GraphicsPipeline>,

    pub viewport: Viewport,

    // Shaders
    deferred_vertex_shader: Arc<ShaderModule>,
    deferred_fragment_shader: Arc<ShaderModule>,
    ambient_vertex_shader: Arc<ShaderModule>,
    ambient_fragment_shader: Arc<ShaderModule>,
    directional_vertex_shader: Arc<ShaderModule>,
    directional_fragment_shader: Arc<ShaderModule>,

    // Variables used during rendering
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub invalid_swapchain: bool,
}
