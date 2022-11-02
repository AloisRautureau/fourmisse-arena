mod rendering_engine;
mod simulation;

use std::net::Shutdown::Write;
use std::time::{Duration, Instant};
use nalgebra_glm::{identity, look_at, perspective, pi, rotate_normalized_axis, TMat4, translate, vec3};
use crate::rendering_engine::{AmbientLightSource, CLEAR_COLOR, ModelViewProjection, Vertex, DirectionalLightSource, deferred_vertex_shader, ambient_fragment_shader, directional_fragment_shader, Model};
use rendering_engine::RenderingEngine;
use simulation::Simulation;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
};
use vulkano::format::ClearValue;
use vulkano::swapchain::{AcquireError, PresentInfo};
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::{swapchain, sync};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

const DEFAULT_TICKS: usize = 100000;

pub fn run_gui(world: String, brains: (String, String), ticks: Option<usize>) {
    // Initial setup
    let event_loop = EventLoop::new();
    let mut rendering_engine = RenderingEngine::init(&event_loop);
    let mut simulation = Simulation::new(&world, &brains.0, &brains.1);
    let mut mvp = ModelViewProjection::default();
    mvp.view = look_at(
        &vec3(0.0, 0.0, 0.01),
        &vec3(0.0, 0.0, 0.0),
        &vec3(0.0, -1.0, 0.0)
    );
    let dimensions: [u32; 2] = rendering_engine.surface.window().inner_size().into();
    mvp.projection = perspective(dimensions[0] as f32 / dimensions[1] as f32, 180.0, 0.01, 100.0);
    mvp.model = translate(&identity(), &vec3(0.0, 0.0, -2.5));

    // Lights
    let ambient_light = AmbientLightSource { color: [1f32, 1f32, 1f32], intensity: 0.1 };
    let directional_light_r = DirectionalLightSource { color: [0.8, 0.1412, 0.1137], position: [-4.0, 0.0, -2.0], intensity: 1.0 };
    let directional_light_g = DirectionalLightSource { color: [0.5961, 0.5922, 0.1020], position: [0.0, -4.0, 1.0], intensity: 1.0 };
    let directional_light_b = DirectionalLightSource { color: [0.2706, 0.5216, 0.5333], position: [4.0, -2.0, -1.0], intensity: 1.0 };

    // Models
    let mut hexagon = Model::load("assets/hexagon.obj", None, true);
    hexagon.translate(&vec3(0.0, 0.0, -2.5));
    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        rendering_engine.device.clone(),
        BufferUsage {
            vertex_buffer: true,
            .. Default::default()
        },
        false,
        hexagon.data().iter().cloned()
    ).unwrap();

    let mvp_pool = CpuBufferPool::<deferred_vertex_shader::ty::MVP>::uniform_buffer(rendering_engine.device.clone());
    let ambient_pool = CpuBufferPool::<ambient_fragment_shader::ty::AmbientLight>::uniform_buffer(rendering_engine.device.clone());
    let directional_pool = CpuBufferPool::<directional_fragment_shader::ty::DirectionalLight>::uniform_buffer(rendering_engine.device.clone());

    // Main loop
    let mut time = Instant::now();
    let mut delta_t = Duration::ZERO;
    let mut ticks_left = ticks.unwrap_or(DEFAULT_TICKS);
    event_loop.run(move |event, _, control_flow| match event {
        // Window should close
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::ExitWithCode(0);
        }
        // Window gets resized
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => rendering_engine.invalid_swapchain = true,
        // Actual gameplay/render loop
        Event::RedrawEventsCleared => {
            delta_t = Instant::now() - time;
            time = Instant::now();

            // Some maintenance operations (GpuFuture cleanup, swapchain recreation, etc)
            rendering_engine
                .previous_frame_end
                .as_mut()
                .take()
                .unwrap()
                .cleanup_finished();
            if rendering_engine.invalid_swapchain {
                let dimensions: [u32; 2] = rendering_engine.surface.window().inner_size().into();
                mvp.projection = perspective(dimensions[0] as f32 / dimensions[1] as f32, 180.0, 0.01, 100.0);
                rendering_engine.recreate_swapchain()
            }

            // Render the current simulation state
            let (image_index, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(rendering_engine.swapchain.clone(), None) {
                    Ok(res) => res,
                    Err(AcquireError::OutOfDate) => {
                        rendering_engine.invalid_swapchain = true;
                        return;
                    }
                    Err(err) => panic!("failed to acquire the next image: {:?}", err),
                };
            if suboptimal {
                rendering_engine.invalid_swapchain = true;
            }

            // Command buffer
            let mut commands = AutoCommandBufferBuilder::primary(
                rendering_engine.device.clone(),
                rendering_engine.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap_or_else(|err| panic!("failed to create command buffer: {:?}", err));
            commands
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![
                            Some(ClearValue::Float(CLEAR_COLOR)),
                            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                            Some(ClearValue::Depth(1.0)),
                        ],
                        ..RenderPassBeginInfo::framebuffer(
                            rendering_engine.framebuffers[image_index].clone(),
                        )
                    },
                    SubpassContents::Inline,
                )
                .unwrap_or_else(|err| panic!("failed to create command buffer: {:?}", err))
                .set_viewport(0, [rendering_engine.viewport.clone()]);

            // DEFERRED PASS
            let mvp_buffer = {
                let elapsed = delta_t.as_secs() as f64 + delta_t.subsec_nanos() as f64 / 1_000_000_000.0;
                let angle = elapsed * pi::<f64>() / 180.0;
                hexagon.rotate(angle as f32 * 30.0, &vec3(0.0, 0.0, 1.0));
                hexagon.rotate(angle as f32 * 20.0, &vec3(1.0, 0.0, 0.0));

                let uniform_data = deferred_vertex_shader::ty::MVP {
                    model: hexagon.model_matrix().into(),
                    view: mvp.view.into(),
                    projection: mvp.projection.into()
                };
                mvp_pool.from_data(uniform_data).unwrap()
            };
            let deferred_layout = rendering_engine.deferred_pipeline
                .layout()
                .set_layouts()
                .get(0)
                .unwrap();
            let deferred_set = PersistentDescriptorSet::new(
                deferred_layout.clone(),
                [
                    WriteDescriptorSet::buffer(0, mvp_buffer.clone()),
                ]
            ).unwrap();
            commands
                .bind_pipeline_graphics(rendering_engine.deferred_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    rendering_engine.deferred_pipeline.layout().clone(),
                    0,
                    deferred_set.clone()
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .next_subpass(SubpassContents::Inline)
                .unwrap();


            // DIRECTIONAL LIGHTS
            let directional_layout = rendering_engine.directional_pipeline
                .layout()
                .set_layouts()
                .get(0)
                .unwrap();

            let directional_buffer = directional_light_r.generate_directional_buffer(&directional_pool);
            let directional_set = PersistentDescriptorSet::new(
                directional_layout.clone(),
                [
                    WriteDescriptorSet::image_view(0, rendering_engine.vertex_color_buffer.clone()),
                    WriteDescriptorSet::image_view(1, rendering_engine.normal_buffer.clone()),
                    WriteDescriptorSet::buffer(2, mvp_buffer.clone()),
                    WriteDescriptorSet::buffer(3, directional_buffer.clone()),
                ]
            ).unwrap();
            commands
                .bind_pipeline_graphics(rendering_engine.directional_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    rendering_engine.directional_pipeline.layout().clone(),
                    0,
                    directional_set.clone()
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap();

            let directional_buffer = directional_light_g.generate_directional_buffer(&directional_pool);
            let directional_set = PersistentDescriptorSet::new(
                directional_layout.clone(),
                [
                    WriteDescriptorSet::image_view(0, rendering_engine.vertex_color_buffer.clone()),
                    WriteDescriptorSet::image_view(1, rendering_engine.normal_buffer.clone()),
                    WriteDescriptorSet::buffer(2, mvp_buffer.clone()),
                    WriteDescriptorSet::buffer(3, directional_buffer.clone()),
                ]
            ).unwrap();
            commands
                .bind_pipeline_graphics(rendering_engine.directional_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    rendering_engine.directional_pipeline.layout().clone(),
                    0,
                    directional_set.clone()
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap();

            let directional_buffer = directional_light_b.generate_directional_buffer(&directional_pool);
            let directional_set = PersistentDescriptorSet::new(
                directional_layout.clone(),
                [
                    WriteDescriptorSet::image_view(0, rendering_engine.vertex_color_buffer.clone()),
                    WriteDescriptorSet::image_view(1, rendering_engine.normal_buffer.clone()),
                    WriteDescriptorSet::buffer(2, mvp_buffer.clone()),
                    WriteDescriptorSet::buffer(3, directional_buffer.clone()),
                ]
            ).unwrap();
            commands
                .bind_pipeline_graphics(rendering_engine.directional_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    rendering_engine.directional_pipeline.layout().clone(),
                    0,
                    directional_set.clone()
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap();

            // AMBIENT LIGHT
            let ambient_layout = rendering_engine.ambient_pipeline
                .layout()
                .set_layouts()
                .get(0)
                .unwrap();
            let ambient_buffer = ambient_light.generate_ambient_buffer(&ambient_pool);
            let ambient_set = PersistentDescriptorSet::new(
                ambient_layout.clone(),
                [
                    WriteDescriptorSet::image_view(0, rendering_engine.vertex_color_buffer.clone()),
                    WriteDescriptorSet::buffer(1, mvp_buffer.clone()),
                    WriteDescriptorSet::buffer(2, ambient_buffer.clone()),
                ]
            ).unwrap();
            commands
                .bind_pipeline_graphics(rendering_engine.ambient_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    rendering_engine.ambient_pipeline.layout().clone(),
                    0,
                    ambient_set.clone()
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            let command_buffer = commands
                .build()
                .unwrap_or_else(|err| panic!("failed to build command buffer: {:?}", err));

            // Actual rendering
            let future = rendering_engine
                .previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(rendering_engine.queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(
                    rendering_engine.queue.clone(),
                    PresentInfo {
                        index: image_index,
                        ..PresentInfo::swapchain(rendering_engine.swapchain.clone())
                    },
                )
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    rendering_engine.previous_frame_end = Some(Box::new(future) as Box<_>)
                }
                Err(FlushError::OutOfDate) => {
                    rendering_engine.invalid_swapchain = true;
                    rendering_engine.previous_frame_end =
                        Some(Box::new(sync::now(rendering_engine.device.clone())) as Box<_>)
                }
                Err(err) => {
                    println!("failed to flush future: {:?}", err);
                    rendering_engine.previous_frame_end =
                        Some(Box::new(sync::now(rendering_engine.device.clone())) as Box<_>)
                }
            }

            // Process a tick of our simulation
            if ticks_left > 0 {
                simulation.process_tick();
                ticks_left -= 1;
                let (red_points, black_points) = simulation.points();
                println!("{} : {}", red_points, black_points)
            }
        }
        _ => {}
    })
}

// Runs one game given a world, brains files, as well as the number of ticks per game
// (defaulting to DEFAULT_TICKS)
pub fn run(world: String, brains: (String, String), ticks: Option<usize>) {
    let mut simulation = Simulation::new(&world, &brains.0, &brains.1);

    for _ in 0..ticks.unwrap_or(DEFAULT_TICKS) {
        simulation.process_tick()
    }

    let (red_points, black_points) = simulation.points();
    if red_points > black_points {
        println!(
            "Red ants won with {} against {} for black ants",
            red_points, black_points
        )
    } else if black_points > red_points {
        println!(
            "Black ants won with {} against {} for red ants",
            black_points, red_points
        )
    } else {
        println!("It's a draw! Both teams got {} points", black_points)
    }
}

// Returns the average score between two brains over a given number of games in a given world
pub fn get_average_score(
    world: String,
    brains: (String, String),
    games: usize,
    ticks: Option<usize>,
) {
    // If the number of games is uneven, we'll play one more
    let games = if games % 2 != 0 { games + 1 } else { games };

    let mut total_score_red = (0, 0);
    let mut total_score_black = (0, 0);
    for g in 0..games {
        let mut simulation = Simulation::new(
            &world,
            if g % 2 == 0 { &brains.0 } else { &brains.1 },
            if g % 2 == 0 { &brains.1 } else { &brains.0 },
        );

        for _ in 0..ticks.unwrap_or(DEFAULT_TICKS) {
            simulation.process_tick()
        }

        let (red_points, black_points) = simulation.points();
        if g % 2 == 0 {
            total_score_red.0 += red_points;
            total_score_black.1 += black_points;
        } else {
            total_score_red.1 += red_points;
            total_score_black.0 += black_points;
        }
    }

    let average_red = (
        total_score_red.0 / (games as u32 / 2),
        total_score_red.1 / (games as u32 / 2),
    );
    let average_black = (
        total_score_black.0 / (games as u32 / 2),
        total_score_black.1 / (games as u32 / 2),
    );
    let average = (
        (total_score_red.0 + total_score_black.0) / games as u32,
        (total_score_red.1 + total_score_black.1) / games as u32,
    );
    println!(
        "Brain {} averaged:\n- {} points as red\n- {} points as black\n- {} points total",
        brains.0, average_red.0, average_black.0, average.0
    );
    println!(
        "Brain {} averaged:\n- {} points as red\n- {} points as black\n- {} points total",
        brains.1, average_red.1, average_black.1, average.1
    );
}
