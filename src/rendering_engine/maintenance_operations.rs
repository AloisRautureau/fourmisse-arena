use crate::rendering_engine::deferred_vertex_shader;
use nalgebra_glm::perspective;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::Pipeline;
use vulkano::swapchain::{SwapchainCreateInfo, SwapchainCreationError};

impl super::RenderingEngine {
    pub fn recreate_viewport_dependant_assets(&mut self) {
        self.recreate_swapchain();
        self.recreate_vp();
    }

    fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.surface.window().inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(res) => res,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(err) => panic!("failed to recreate our swapchain: {:?}", err),
        };

        self.swapchain = new_swapchain;
        let (
            new_framebuffers,
            new_vertex_color_buffer,
            new_normal_buffer,
            new_frag_pos_buffer,
            new_specular_buffer,
        ) = super::init::window_size_dependent_setup(
            self.device.clone(),
            &new_images,
            self.render_pass.clone(),
            &mut self.viewport,
        );
        self.framebuffers = new_framebuffers;
        self.vertex_color_buffer = new_vertex_color_buffer;
        self.normal_buffer = new_normal_buffer;
        self.frag_pos_buffer = new_frag_pos_buffer;
        self.specular_buffer = new_specular_buffer;
    }

    fn recreate_vp(&mut self) {
        let dimensions: [f32; 2] = self.surface.window().inner_size().into();
        self.vp_matrix.projection = perspective(dimensions[0] / dimensions[1], 100.0, 0.01, 1024.0);
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

        let deferred_layout = self
            .deferred_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        self.vp_descriptor_set = PersistentDescriptorSet::new(
            deferred_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())],
        )
        .unwrap();
    }
}
