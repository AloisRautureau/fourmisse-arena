pub mod ant;
pub mod instruction;
pub mod map;

use crate::rendering_engine::{
    LightSource, Material, RenderingEngine, ResourceHandle, ResourceHandler, ResourceVec,
};
use crate::simulation::ant::Colour;
use crate::simulation::instruction::load_instructionset;
use crate::simulation::map::AntRef;
use instruction::InstructionSet;
use map::Map;
use nalgebra_glm::{identity, make_vec3, pi, rotate, translate, vec3, vec3_to_vec4, TVec3};
pub use map::Pov;

const HEXAGON_RADIUS: f32 = 1_f32;
const HEXAGON_HEIGHT: f32 = 2_f32 * HEXAGON_RADIUS;
const HEXAGON_WIDTH: f32 = 1.732050807568 * HEXAGON_RADIUS; // 3_f32.sqrt() * HEXAGON_RADIUS

// Day/night cycle simulation constants
const MAX_SIMULATION_TIME: u32 = 60 * 60 * 24;
const NOON_THRESHOLD: u32 = MAX_SIMULATION_TIME / 2;
const EVENING_THRESHOLD: u32 = 2 * MAX_SIMULATION_TIME / 3;
const NOON_LIGHTING: LightSource = LightSource {
    vector: [0f32; 4],
    color: [1f32; 3],
};
const EVENING_LIGHTING: LightSource = LightSource {
    vector: [0f32; 4],
    color: [0.83921, 0.364706, 0.054902],
};
const NIGHT_LIGHTING: LightSource = LightSource {
    vector: [0f32; 4],
    color: [0.51373, 0.64706, 0.59608],
};

// Represents the current state of a simulation
pub struct Simulation {
    pub ants: Vec<AntRef>,
    pub map: Map,
    instructions: [InstructionSet; 2],
    in_simulation_time: u32, // MAX 86400, time in seconds per day
    resource_handler: ResourceHandler,
    tile_model_handle: ResourceHandle,
    red_ant_model_handle: ResourceHandle,
    black_ant_model_handle: ResourceHandle,
    food_model_handle: ResourceHandle,
}
impl Simulation {
    // Creates a new simulation, loading the needed resources as needed
    pub fn new(map_path: &str, red_brain_path: &str, black_brain_path: &str) -> Self {
        // Load up resources
        let mut resource_handler = ResourceHandler::default();
        let red_ant_model = resource_handler.models.load("assets/ant.obj");
        resource_handler
            .models
            .set_colour(&red_ant_model, Colour::Red.rgb());
        let black_ant_model = resource_handler.models.load("assets/ant.obj");
        resource_handler
            .models
            .set_colour(&black_ant_model, Colour::Black.rgb());
        let food_model = resource_handler.models.load("assets/food.obj");
        resource_handler
            .models
            .set_colour(&food_model, [0.98039, 0.841176, 0.184314]);

        // Then create the actual map
        let hexagon_model = resource_handler.models.load("assets/hexagon.obj");
        let (map, ants) = Map::load_file(map_path, hexagon_model, &resource_handler);
        Self {
            ants,
            map,
            instructions: [
                load_instructionset(red_brain_path),
                load_instructionset(black_brain_path),
            ],
            resource_handler,

            in_simulation_time: MAX_SIMULATION_TIME / 2,
            tile_model_handle: hexagon_model,
            red_ant_model_handle: red_ant_model,
            black_ant_model_handle: black_ant_model,
            food_model_handle: food_model,
        }
    }

    // Each ant executes its current instruction, then
    // surrounded ants are killed
    pub fn process_tick(&mut self) {
        // Each ant moves
        for ant_ref in &self.ants {
            let ant = &mut ant_ref.lock().unwrap();
            let instruction_set = { &self.instructions[ant.colour.as_index()] };
            ant.process_tick(&mut self.map, instruction_set);
            self.map.cleanup_killed_ants();
        }

        // We only need to keep ants that appear on the map, so we recreate our ants vector
        self.ants = self.map.ants();

        self.in_simulation_time = (self.in_simulation_time + 1) % MAX_SIMULATION_TIME;
    }

    // Returns the current food units in each nest
    pub fn score(&self) -> (u32, u32) {
        self.map.score()
    }

    // Renders the next frame of the simulation, given an interpolation ratio
    // to avoid stuttering
    pub fn render(&mut self, interpolation_ratio: f32, pov: Pov, renderer: &mut RenderingEngine) {
        renderer.begin();
        self.map
            .render(pov, renderer, self.tile_model_handle, self.food_model_handle, &self.resource_handler);

        // Render each ant
        for ant_ref in self.ants.iter_mut() {
            let ant = &mut ant_ref.lock().unwrap();

            ant.interpolate_state(interpolation_ratio);
            let ant_model_handle = if ant.colour == Colour::Red { self.red_ant_model_handle } else { self.black_ant_model_handle };
            ant.render(
                renderer,
                ant_model_handle,
                self.food_model_handle,
                &self.resource_handler,
            );
        }

        // Lighting
        renderer.calculate_ambient_lighting();

        // Light emanating from food
        self.map.render_light(renderer);
        // Light emanating from ants
        for ant_ref in self.ants.iter_mut() {
            let ant = &mut ant_ref.lock().unwrap();
            ant.render_light(renderer);
        }

        // The color and position of the directional light change depending on the time of day,
        // interpolating between the NOON_LIGHTING, EVENING_LIGHTING, and NIGHT_LIGHTING respectively
        let interpolate_lights = |l1: LightSource, l2: LightSource, ratio: f32| {
            let color = make_vec3(&l1.color) * ratio + make_vec3(&l2.color) * (1_f32 - ratio);
            color.data.0[0]
        };
        let light_angle = pi::<f64>() / MAX_SIMULATION_TIME as f64 * self.in_simulation_time as f64;
        let (offset_x, offset_y) = self.map.size;
        let (offset_x, offset_y) = (
            offset_x as f32 / 2f32 * HEXAGON_WIDTH,
            offset_y as f32 / 2f32 * HEXAGON_HEIGHT,
        );
        let direction = [
            -offset_y + 100f32,
            -100f32 * light_angle.sin() as f32,
            -offset_x - 100f32 * light_angle.cos() as f32,
            0.0,
        ];
        renderer.add_directional_light(&if self.in_simulation_time < NOON_THRESHOLD {
            // Morning, so we interpolate between NIGHT_LIGHTING and NOON_LIGHTING
            let noon_light_ratio = self.in_simulation_time as f32 / NOON_THRESHOLD as f32;
            let color = interpolate_lights(NOON_LIGHTING, NIGHT_LIGHTING, noon_light_ratio);
            LightSource {
                vector: direction,
                color,
            }
        } else if self.in_simulation_time < EVENING_THRESHOLD {
            // Afternoon, so we interpolate between NOON_LIGHTING and EVENING_LIGHTING
            let evening_light_ratio = (self.in_simulation_time - NOON_THRESHOLD) as f32
                / (EVENING_THRESHOLD - NOON_THRESHOLD) as f32;
            let color = interpolate_lights(EVENING_LIGHTING, NOON_LIGHTING, evening_light_ratio);
            LightSource {
                vector: direction,
                color,
            }
        } else {
            // Night, so we interpolate between EVENING_LIGHTING and NIGHT_LIGHTING
            let night_light_ratio = (self.in_simulation_time - EVENING_THRESHOLD) as f32
                / (MAX_SIMULATION_TIME - EVENING_THRESHOLD) as f32;
            let color = interpolate_lights(NIGHT_LIGHTING, EVENING_LIGHTING, night_light_ratio);
            LightSource {
                vector: direction,
                color,
            }
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

    // Renders the geometry of a piece of food
    fn render_food_piece(
        renderer: &mut RenderingEngine,
        food_model_handle: ResourceHandle,
        resource_handler: &ResourceHandler,
        position: TVec3<f32>,
        rotation: f32,
    ) {
        let mut model_matrix = translate(&identity(), &position);
        model_matrix = rotate(&model_matrix, rotation, &vec3(0f32, 1f32, 0f32));
        renderer.add_model(
            resource_handler
                .models
                .fetch_model_vertices(&food_model_handle),
            (model_matrix, model_matrix),
            &Material {
                shininess: 0.0,
                specular_intensity: 0.0,
            },
        )
    }
    // Renders the light emanating from a piece of food
    fn render_food_light(renderer: &mut RenderingEngine, position: TVec3<f32>) {
        let mut position = vec3_to_vec4(&position);
        position.w = 1f32;
        let position = position.data.0[0];
        renderer.add_directional_light(&LightSource {
            color: [0.98039, 0.841176, 0.184314],
            vector: position,
        })
    }
}
