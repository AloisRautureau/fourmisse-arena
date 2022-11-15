mod rendering_engine;
mod simulation;
mod ecs;
mod resources;

use crate::rendering_engine::{RenderStage, Vertex};
use nalgebra_glm::{pi, rotation, vec3, vec4, vec4_to_vec3};
use rendering_engine::RenderingEngine;
use simulation::Simulation;
use std::cmp::Ordering;
use std::time::Instant;
use winit::dpi::PhysicalPosition;
use winit::event::KeyboardInput;
use winit::event::{DeviceEvent, ElementState, Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use crate::simulation::Pov;

const DEFAULT_TICKS: usize = 100000;

pub fn run_gui(world: String, brains: (String, String), ticks: Option<usize>) {
    // Initial setup
    let event_loop = EventLoop::new();
    let mut rendering_engine = RenderingEngine::init(&event_loop);
    let mut simulation = Simulation::new(&world, &brains.0, &brains.1);

    // Input state
    let mut button_pressed = None;

    // Idea taken from https://cs.pomona.edu/classes/cs181g/notes/controlling-time.html, well
    // documented in the literature
    // The simulation updates a fixed number of times per second, and frames are then interpolated.
    // This allows the game to run the same whether it's on slow or faster hardware, while avoiding
    // wasted clock cycles
    let mut sim_running = false;
    let mut tps = 256;
    let tick_dt = |tps: usize| 1_f32 / tps as f32;

    let mut time_acc = 0.0_f32;
    let mut previous_tick = Instant::now();
    let mut simulation_ticks_left = ticks.unwrap_or(DEFAULT_TICKS);

    let mut last_frame_t = Instant::now();
    let mut frame_count = 0;

    let mut current_pov = Pov::Both;

    // Main loop
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
        } => rendering_engine.render_stage = RenderStage::Invalid,
        Event::DeviceEvent { event, .. } => {
            match event {
                DeviceEvent::Button {
                    button,
                    state: ElementState::Pressed,
                } => button_pressed = Some(button),
                DeviceEvent::Button {
                    button,
                    state: ElementState::Released,
                } if Some(button) == button_pressed => button_pressed = None,
                DeviceEvent::MouseMotion { delta: (dx, dy) } if button_pressed.is_some() => {
                    // Necessary since we're in an isometric view
                    let rota = rotation(pi::<f32>() / 3_f32, &vec3(0.0, 1.0, 0.0));
                    let translation = rota * vec4(dx as f32, 0_f32, -dy as f32, 1_f32);
                    rendering_engine.move_camera(&(vec4_to_vec3(&translation) / 10_f32))
                }
                DeviceEvent::MouseWheel { delta } => match delta {
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => rendering_engine
                        .move_camera(&(vec3(y as f32, y as f32, y as f32) / 100_f32)),
                    MouseScrollDelta::LineDelta(_, y) => {
                        rendering_engine.move_camera(&(vec3(y as f32, y as f32, y as f32) / 10_f32))
                    }
                },
                DeviceEvent::Key(KeyboardInput {
                    scancode,
                    state: ElementState::Pressed,
                    ..
                }) => {
                    match scancode {
                        103 => tps *= 2,           // Keyboard up
                        108 => tps /= 2,           // Keyboard down
                        57 => sim_running ^= true, // spacebar
                        48 => current_pov = if current_pov == Pov::BlackAnts { Pov::Both } else { Pov::BlackAnts }, // b
                        19 => current_pov = if current_pov == Pov::RedAnts { Pov::Both } else { Pov::RedAnts }, // r
                        _ => println!("{:?}", scancode),
                    }
                }
                _ => (),
            }
        }
        // Actual gameplay/render loop
        Event::RedrawEventsCleared => {
            if sim_running && simulation_ticks_left == 0 {
                sim_running = false;
                let (red_points, black_points) = simulation.score();
                println!("{} : {}", red_points, black_points)
            }

            // Ticks handling
            // We always process a fixed number of ticks each second, but
            // our frames still refresh as fast as possible
            // Since we can now update the simulation multiple times per second, we may need
            // to interpolate between frames
            let previous_tick_elapsed = previous_tick.elapsed().as_secs_f32();
            previous_tick = Instant::now();
            let interpolation_ratio = if sim_running {
                time_acc += previous_tick_elapsed;
                while time_acc >= tick_dt(tps) && simulation_ticks_left > 0 {
                    simulation.process_tick();
                    simulation_ticks_left -= 1;
                    time_acc -= tick_dt(tps);
                }
                time_acc / tick_dt(tps)
            } else {
                previous_tick = Instant::now();
                1_f32
            };

            // We then render our simulation state using the interpolation ratio
            simulation.render(interpolation_ratio, current_pov, &mut rendering_engine);
            frame_count += 1;
            if last_frame_t.elapsed().as_secs() >= 1 {
                println!("fps: {}", frame_count);
                frame_count = 0;
                last_frame_t = Instant::now();
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

    let (red_points, black_points) = simulation.score();
    match red_points.cmp(&black_points) {
        Ordering::Greater => println!(
            "Red ants won with {} against {} for black ants",
            red_points, black_points
        ),
        Ordering::Less => println!(
            "Black ants won with {} against {} for red ants",
            black_points, red_points
        ),
        _ => println!("It's a draw! Both teams got {} points", black_points),
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

        let (red_points, black_points) = simulation.score();
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
