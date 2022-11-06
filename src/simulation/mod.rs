pub mod ant;
pub mod instruction;
pub mod map;

use crate::rendering_engine::{
    DirectionalLightSource, RenderingEngine, ResourceHandle, ResourceHandler, ResourceVec,
};
use crate::simulation::ant::{Ant, Colour};
use crate::simulation::instruction::load_instructionset;
use crate::simulation::map::AntRef;
use instruction::InstructionSet;
use map::Map;
use nalgebra_glm::{vec3, TVec3};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::rc::Rc;

const HEXAGON_RADIUS: f32 = 1_f32;
const HEXAGON_HEIGHT: f32 = 2_f32 * HEXAGON_RADIUS;
const HEXAGON_WIDTH: f32 = 1.732050807568 * HEXAGON_RADIUS; // 3_f32.sqrt() * HEXAGON_RADIUS

// Represents the current state of a simulation
pub struct Simulation {
    pub ants: Vec<AntRef>,
    pub map: Map,
    instructions: [InstructionSet; 2],
    resource_handler: ResourceHandler,
    tile_model_handle: ResourceHandle,
    ant_model_handle: ResourceHandle,
}
impl Simulation {
    // Creates a new simulation, loading the needed resources as needed
    pub fn new(map_path: &str, red_brain_path: &str, black_brain_path: &str) -> Self {
        // Load up resources
        let mut resource_handler = ResourceHandler::default();
        let ant_model = resource_handler.models.load("assets/ant.obj");
        let hexagon_model = resource_handler.models.load("assets/hexagon.obj");
        // Then create the actual map
        let (map, ants) = Map::load_file(map_path);
        Self {
            ants,
            map,
            instructions: [
                load_instructionset(red_brain_path),
                load_instructionset(black_brain_path),
            ],
            resource_handler,

            tile_model_handle: hexagon_model,
            ant_model_handle: ant_model,
        }
    }

    // Each ant executes its current instruction, then
    // surrounded ants are killed
    pub fn process_tick(&mut self) {
        // Each ant moves
        for ant in &mut self.ants {
            let ant = Rc::clone(ant);
            let instruction_set = {
                let a: &RefCell<Ant> = ant.borrow();
                &self.instructions[a.borrow().colour.as_index()]
            };
            Ant::process_tick(ant, &mut self.map, instruction_set)
        }

        // Surrounded ants are killed
        // TODO
    }

    // Returns the current food units in each nest
    pub fn points(&self) -> (u32, u32) {
        self.map.points()
    }

    // Renders the next frame of the simulation, given an interpolation ratio
    // to avoid stuttering
    pub fn render(&mut self, interpolation_ratio: f32, renderer: &mut RenderingEngine) {
        renderer.begin();
        self.map
            .render(renderer, self.tile_model_handle, &self.resource_handler);

        // Render each ant
        for ant_ref in self.ants.iter_mut() {
            let ant: &RefCell<Ant> = ant_ref.borrow_mut();
            let mut ant = ant.borrow_mut();

            ant.interpolate_state(interpolation_ratio);
            ant.render(renderer, self.ant_model_handle, &self.resource_handler);
        }

        // Lighting
        renderer.calculate_ambient_lighting();
        renderer.add_directional_light(&DirectionalLightSource {
            color: [1_f32; 3],
            position: [100_f32, 200_f32, 100_f32],
            intensity: 1_f32,
        });
        renderer.end()
    }

    // Given a discrete position, returns a render position
    fn render_position(position: (usize, usize)) -> TVec3<f32> {
        let (render_x, render_y) = (
            position.0 as f32 * HEXAGON_WIDTH
                + if position.1 % 2 != 0 {
                    HEXAGON_WIDTH / 2_f32
                } else {
                    0_f32
                },
            position.1 as f32 * HEXAGON_HEIGHT * 0.75,
        );
        vec3(render_y, 0.0, render_x)
    }
}
