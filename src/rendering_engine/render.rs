use crate::rendering_engine::materials::Material;
use crate::rendering_engine::resource_handler::ResourceHandler;
use crate::rendering_engine::{deferred_fragment_shader, deferred_vertex_shader, DirectionalLightSource, RenderStage, RenderingEngine, ResourceHandle, directional_fragment_shader};
use nalgebra_glm::TMat4;
use std::mem;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::format::ClearValue;
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::swapchain::{acquire_next_image, AcquireError, PresentInfo};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};

// Implements actual rendering commands
impl RenderingEngine {
    // Signals that we want to draw a new frame
    pub fn begin(&mut self) {
        // Enforce the renderer to be in a "Stopped" state before drawing a new frame
        match self.render_stage {
            RenderStage::Stopped => self.render_stage = RenderStage::Deferred,
            _ => {
                self.wrong_stage();
                return;
            }
        }

        // Some maintenance operations (GpuFuture cleanup, swapchain recreation, etc)
        let mut previous_frame_end = self.previous_frame_end.take().unwrap();
        previous_frame_end.cleanup_finished();
        self.previous_frame_end = Some(previous_frame_end);

        match acquire_next_image(self.swapchain.clone(), None) {
            Ok((image_index, suboptimal, future_handle)) => {
                // Our current swapchain may be suboptimal, therefore we recreate it
                if suboptimal {
                    self.render_stage = RenderStage::Invalid;
                    return;
                }
                self.image_index = image_index;
                self.future_handle = Some(future_handle);
            }
            Err(AcquireError::OutOfDate) => {
                self.render_stage = RenderStage::Invalid;
                return;
            }
            Err(err) => panic!("could not start rendering frame: {:?}", err),
        };

        let clear_values =  vec![
            Some([0_f32; 4].into()), // Colour
            Some([0_f32; 2].into()), // Depth image,
            Some([0_f32; 4].into()), // Depth image,
            Some([0_f32; 4].into()), // Depth image,
            Some([0_f32; 2].into()), // Specular light
            Some([1_f32, 0_f32].into())
        ];
        let mut commands = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap_or_else(|err| panic!("failed to create command buffer builder: {:?}", err));
        commands
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[self.image_index].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap_or_else(|err| panic!("failed to begin render pass: {:?}", err));

        self.commands = Some(commands);
    }

    // Adds a model to our world
    pub fn add_model(
        &mut self,
        model_handle: ResourceHandle,
        resource_handler: &ResourceHandler,
        model_transforms: (TMat4<f32>, TMat4<f32>),
        material: &Material,
    ) {
        // Check whether we're in the right state
        match self.render_stage {
            RenderStage::Deferred => (),
            _ => {
                self.wrong_stage();
                return;
            }
        }

        // Binding our uniform buffers
        let model_buffer = {
            let (model_transform, normals_transform) = model_transforms;
            let uniform_data = deferred_vertex_shader::ty::Model {
                model_transform: model_transform.into(),
                normals_transform: normals_transform.into(),
            };
            self.model_buffer_pool
                .from_data(uniform_data)
                .unwrap_or_else(|err| panic!("failed to allocate model buffer: {:?}", err))
        };
        let material_buffer = {
            let uniform_data = deferred_fragment_shader::ty::Material {
                color: material.colour,
                shininess: material.shininess
            };
            self.material_buffer_pool
                .from_data(uniform_data)
                .unwrap_or_else(|err| panic!("failed to allocate material buffer: {:?}", err))
        };
        let model_layout = self
            .deferred_pipeline
            .layout()
            .set_layouts()
            .get(1)
            .unwrap();
        let model_descriptor_set = PersistentDescriptorSet::new(
            model_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, model_buffer),
                WriteDescriptorSet::buffer(1, material_buffer),
            ],
        )
        .unwrap();

        let mut commands = self.commands.take().unwrap();
        commands
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.deferred_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.deferred_pipeline.layout().clone(),
                0,
                (self.vp_descriptor_set.clone(), model_descriptor_set),
            );
        // If we try to render a set of vertices which are not yet bound, we must prepare a buffer and bind them
        if self.bound_model_handle != Some(model_handle) {
            let vertex_buffer = CpuAccessibleBuffer::from_iter(
                self.device.clone(),
                BufferUsage {
                    vertex_buffer: true,
                    ..Default::default()
                },
                false,
                resource_handler
                    .models
                    .fetch_model_vertices(&model_handle)
                    .iter()
                    .cloned(),
            )
            .unwrap_or_else(|err| panic!("failed to create vertex buffer: {:?}", err));
            self.bound_model_handle = Some(model_handle);

            commands.bind_vertex_buffers(0, vertex_buffer.clone());
        }
        commands
            .draw(
                resource_handler
                    .models
                    .fetch_model_vertices(&model_handle)
                    .len() as u32,
                1,
                0,
                0,
            )
            .unwrap_or_else(|err| panic!("failed to bind vertex buffer: {:?}", err));
        self.commands = Some(commands)
    }

    // Calculates ambient lighting for our world
    // Calling this switches the state of the renderer to the lighting render pass, calls to
    // add_model() placed between this and end() will be invalid
    pub fn calculate_ambient_lighting(&mut self) {
        match self.render_stage {
            RenderStage::Deferred => self.render_stage = RenderStage::Ambient,
            RenderStage::Ambient => return, // No need to do this twice
            _ => {
                self.wrong_stage();
                return;
            }
        }

        let ambient_layout = self.ambient_pipeline.layout().set_layouts().get(0).unwrap();
        let ambient_descriptor_set = PersistentDescriptorSet::new(
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.vertex_color_buffer.clone()),
                WriteDescriptorSet::buffer(1, self.ambient_buffer.clone()),
            ],
        )
        .unwrap();

        // Adding to our command queue
        let mut commands = self.commands.take().unwrap();
        commands
            .next_subpass(SubpassContents::Inline)
            .unwrap_or_else(|err| panic!("failed to switch subpasses: {:?}", err))
            .bind_pipeline_graphics(self.ambient_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.ambient_pipeline.layout().clone(),
                0,
                ambient_descriptor_set,
            )
            .set_viewport(0, [self.viewport.clone()])
            .bind_vertex_buffers(0, self.viewport_span_buffer.clone())
            .draw(self.viewport_span_buffer.len() as u32, 1, 0, 0)
            .unwrap_or_else(|err| panic!("failed to bind ambient pipeline: {:?}", err));
        self.commands = Some(commands)
    }

    // Adds a directional light source to our world
    pub fn add_directional_light(&mut self, directional_light: &DirectionalLightSource) {
        match self.render_stage {
            RenderStage::Ambient => self.render_stage = RenderStage::Directional,
            RenderStage::Directional => (),
            _ => {
                self.wrong_stage();
                return;
            }
        }

        let camera_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage {
                uniform_buffer: true,
                .. Default::default()
            },
            false,
            directional_fragment_shader::ty::Camera {
                position: self.vp_matrix.camera_position.into()
            }
        ).unwrap();
        let directional_buffer =
            directional_light.generate_directional_buffer(&self.directional_buffer_pool);
        let directional_layout = self
            .directional_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let directional_descriptor_set = PersistentDescriptorSet::new(
            directional_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.vertex_color_buffer.clone()),
                WriteDescriptorSet::image_view(1, self.normal_buffer.clone()),
                WriteDescriptorSet::image_view(2, self.frag_pos_buffer.clone()),
                WriteDescriptorSet::image_view(3, self.specular_buffer.clone()),
                WriteDescriptorSet::buffer(4, camera_buffer.clone()),
                WriteDescriptorSet::buffer(5, directional_buffer.clone())
            ],
        )
        .unwrap();

        // Adding to our command queue
        let mut commands = self.commands.take().unwrap();
        commands
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.directional_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.directional_pipeline.layout().clone(),
                0,
                directional_descriptor_set,
            )
            // We don't need to bind the viewport_span_buffer since we've done so in the ambient lighting stage
            .draw(self.viewport_span_buffer.len() as u32, 1, 0, 0)
            .unwrap_or_else(|err| panic!("failed to bind directional light: {:?}", err));
        self.commands = Some(commands)
    }

    pub fn end(&mut self) {
        // Make sure we've just finished applying lighting to out world
        match self.render_stage {
            RenderStage::Directional => (),
            _ => {
                self.wrong_stage();
                return;
            }
        }

        // Build out command buffer
        let mut commands = self.commands.take().unwrap();
        commands
            .end_render_pass()
            .unwrap_or_else(|err| panic!("failed to end render pass: {:?}", err));
        let command_buffer = commands
            .build()
            .unwrap_or_else(|err| panic!("failed to build command buffer: {:?}", err));

        // Fetch our handles
        let future_handle = self.future_handle.take().unwrap();
        let mut local_future_handle: Option<Box<dyn GpuFuture>> = None;
        mem::swap(&mut local_future_handle, &mut self.previous_frame_end);

        // Finally send the command buffer for the GPU to execute
        let future = local_future_handle
            .take()
            .unwrap()
            .join(future_handle)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap_or_else(|err| panic!("failed to send command buffer to the GPU: {:?}", err))
            .then_swapchain_present(
                self.queue.clone(),
                PresentInfo {
                    index: self.image_index,
                    ..PresentInfo::swapchain(self.swapchain.clone())
                },
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future_handle) => self.previous_frame_end = Some(Box::new(future_handle) as Box<_>),
            Err(FlushError::OutOfDate) => {
                self.recreate_viewport_dependant_assets();
                self.previous_frame_end =
                    Some(Box::new(sync::now(self.device.clone())) as Box<dyn GpuFuture>)
            }
            Err(err) => {
                println!("failed to flush future: {:?}", err);
                self.previous_frame_end =
                    Some(Box::new(sync::now(self.device.clone())) as Box<dyn GpuFuture>)
            }
        }
        self.commands = None;
        self.render_stage = RenderStage::Stopped;
    }

    // Applies the right corrections depending on which wrong state we're in
    // Only call this after making sure we're in a wrong stage
    fn wrong_stage(&mut self) {
        if self.render_stage == RenderStage::Invalid {
            self.recreate_viewport_dependant_assets()
        }
        self.commands = None;
        self.render_stage = RenderStage::Stopped
    }
}
