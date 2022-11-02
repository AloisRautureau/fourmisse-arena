use vulkano::swapchain::{SwapchainCreateInfo, SwapchainCreationError};

impl super::RenderingEngine {
    pub fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.surface.window().inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(res) => res,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(err) => panic!("failed to recreate our swapchain: {:?}", err),
        };

        self.swapchain = new_swapchain;
        let (new_framebuffers, new_vertex_color_buffer, new_normal_buffer) = super::init::window_size_dependent_setup(
            self.device.clone(),
            &new_images,
            self.render_pass.clone(),
            &mut self.viewport,
        );
        self.framebuffers = new_framebuffers;
        self.vertex_color_buffer = new_vertex_color_buffer;
        self.normal_buffer = new_normal_buffer;

        self.invalid_swapchain = false;
    }
}
