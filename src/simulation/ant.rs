use super::instruction::{Instruction, Instruction::*, InstructionSet};
use super::map::Map;
use crate::rendering_engine::{LightSource, Material, RenderingEngine, ResourceHandle, ResourceHandler};
use crate::simulation::instruction::{SenseDirection, TurnDirection};
use crate::simulation::map::AntRef;
use crate::simulation::{Simulation, HEXAGON_HEIGHT, HEXAGON_RADIUS, HEXAGON_WIDTH};
use nalgebra_glm::{identity, inverse_transpose, pi, rotate_normalized_axis, translate, vec3, TMat4, TVec3, vec3_to_vec4, make_vec3};
use rand::Rng;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Colour {
    Red,
    Black,
}
impl Colour {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Red => Self::Black,
            _ => Self::Red,
        }
    }
    pub fn as_index(&self) -> usize {
        match self {
            Self::Red => 0,
            _ => 1,
        }
    }
}
impl Default for Colour {
    fn default() -> Self {
        Self::Red
    }
}
impl Colour {
    pub fn rgb(self) -> [f32; 3] {
        match self {
            Colour::Red => [0.8, 0.1412, 0.1137],
            Colour::Black => [0.2353, 0.2196, 0.2118],
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum CardinalDirection {
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}
impl Default for CardinalDirection {
    fn default() -> Self {
        Self::East
    }
}
impl CardinalDirection {
    pub fn right(self) -> Self {
        match self {
            Self::West => Self::NorthWest,
            Self::NorthWest => Self::NorthEast,
            Self::NorthEast => Self::East,
            Self::East => Self::SouthEast,
            Self::SouthEast => Self::SouthWest,
            Self::SouthWest => Self::West,
        }
    }

    pub fn left(self) -> Self {
        match self {
            Self::West => Self::SouthWest,
            Self::SouthWest => Self::SouthEast,
            Self::SouthEast => Self::East,
            Self::East => Self::NorthEast,
            Self::NorthEast => Self::NorthWest,
            Self::NorthWest => Self::West,
        }
    }

    pub fn as_angle(&self) -> f32 {
        match self {
            Self::West => pi::<f32>(),
            Self::East => 0_f32,
            Self::NorthWest => -2_f32 * pi::<f32>() / 3_f32,
            Self::NorthEast => -pi::<f32>() / 3_f32,
            Self::SouthWest => 2_f32 * pi::<f32>() / 3_f32,
            Self::SouthEast => pi::<f32>() / 3_f32,
        }
    }
}

// Completely represents one ant
#[derive(Debug)]
pub struct Ant {
    pub id: usize,
    pub colour: Colour,
    pub position: (usize, usize),
    pub has_food: bool,
    direction: CardinalDirection,

    current_instruction: usize,
    cooldown: usize,

    // Rendering variables
    pub material: Material,
    render_pos: TVec3<f32>,
    render_rot: f32,
    model_matrix: TMat4<f32>,
    normals_matrix: TMat4<f32>,
    should_update_matrices: bool,
}
impl Ant {
    // Creates a new ant of the given colour
    pub fn new(id: usize, colour: Colour, position: (usize, usize)) -> Self {
        Self {
            id,
            colour,
            position,
            has_food: false,
            direction: CardinalDirection::default(),

            current_instruction: 0,
            cooldown: 0,

            material: Material {
                colour: colour.rgb(),
                shininess: 128.0,
                specular_intensity: 1.0
            },
            render_pos: Simulation::render_position(position),
            render_rot: CardinalDirection::default().as_angle(),

            model_matrix: identity(),
            normals_matrix: identity(),
            should_update_matrices: true,
        }
    }

    // Processes one tick, executing a command if the ant is off cooldown, and
    // reducing said cooldown by 1
    // Returns a boolean, indicating whether the ant has moved
    pub fn process_tick(&mut self, map: &mut Map, instructions: &InstructionSet) -> bool {
        if self.cooldown == 0 {
            let current_instruction = instructions
                .get(self.current_instruction)
                .expect("Instruction count is out of bounds");
            self.exec(current_instruction, map);
            matches!(current_instruction, Instruction::Move(_))
        } else {
            self.cooldown -= 1;
            false
        }
    }

    // Executes a given instruction, ant's state and map
    // The instruction can change the ant's state
    // Returns the index of the next instruction
    fn exec(&mut self, instruction: &Instruction, map: &mut Map) {
        self.current_instruction += 1;
        match *instruction {
            Sense(dir, true_label, false_label, cond) => {
                // Calculates the target cell's index
                let cell = self.target_cell(dir);
                // Then checks the given condition and change the current instruction
                // accordingly
                self.current_instruction = if map.check_condition(cond, self.colour, cell) {
                    true_label
                } else {
                    false_label
                }
            }
            Mark(i) => {
                map.mark_pheromone(self.position, i, self.colour);
            }
            Unmark(i) => {
                map.unmark_pheromone(self.position, i, self.colour);
            }
            Pickup(fail_label) => {
                if !self.has_food && map.pickup_food(self.position) {
                    self.has_food = true;
                } else {
                    self.current_instruction = fail_label
                }
            }
            Drop => {
                if self.has_food {
                    map.drop_food(self.position);
                    self.has_food = false
                }
            }
            Turn(TurnDirection::Left) => {
                let next_direction = self.direction.left();
                self.direction = next_direction;
            }
            Turn(TurnDirection::Right) => {
                let next_direction = self.direction.right();
                self.direction = next_direction;
            }
            Move(fail_label) => {
                let from = self.position;
                let to = self.target_cell(SenseDirection::Ahead);
                if map.move_to(self.colour, from, to) {
                    self.position = to;
                    self.cooldown = 14;
                } else {
                    self.current_instruction = fail_label
                }
            }
            Flip(p, success_label, failure_label) => {
                let rng = rand::thread_rng().gen_range(0..p);
                self.current_instruction = if rng == 0 {
                    success_label
                } else {
                    failure_label
                }
            }
            Goto(label) => self.current_instruction = label,
        }
    }

    fn target_cell(&self, direction: SenseDirection) -> (usize, usize) {
        let (x, y) = self.position;
        let sense_direction = match direction {
            SenseDirection::Right => self.direction.right(),
            SenseDirection::Left => self.direction.left(),
            SenseDirection::Here => return self.position,
            _ => self.direction,
        };
        match sense_direction {
            CardinalDirection::West => (x - 1, y),
            CardinalDirection::NorthEast => (x + 1, y - 1),
            CardinalDirection::NorthWest => (x - 1, y - 1),
            CardinalDirection::East => (x + 1, y),
            CardinalDirection::SouthEast => (x + 1, y + 1),
            CardinalDirection::SouthWest => (x - 1, y + 1),
        }
    }

    // RENDERING
    pub fn render(
        &mut self,
        renderer: &mut RenderingEngine,
        ant_model_handle: ResourceHandle,
        food_model_handle: ResourceHandle,
        resource_handler: &ResourceHandler,
    ) {
        renderer.add_model(
            ant_model_handle,
            resource_handler,
            self.model_matrices(),
            &self.material,
        );

        if self.has_food {
            Simulation::render_food_piece(
                renderer,
                food_model_handle,
                resource_handler,
                self.render_pos + vec3(0f32, 0.3, 0f32),
                self.render_rot
            )
        }
    }
    pub fn render_light(&self, renderer: &mut RenderingEngine) {
        let mut vector = vec3_to_vec4(&self.render_pos);
        vector.w = 1f32;
        let vector = vector.data.0[0];
        renderer.add_directional_light(&LightSource {
            color: [0.1; 3],
            vector,
        });

        if self.has_food {
            Simulation::render_food_light(renderer, self.render_pos + vec3(0f32, 0.3, 0f32));
        }
    }

    // Returns both the model transformation matrix, as well as the corresponding normal
    // transformation
    pub fn model_matrices(&mut self) -> (TMat4<f32>, TMat4<f32>) {
        if self.should_update_matrices {
            let translation_matrix = translate(&identity(), &self.render_pos);
            let rotation_matrix =
                rotate_normalized_axis(&identity(), self.render_rot, &vec3(0_f32, 1_f32, 0_f32));

            self.model_matrix = translation_matrix * rotation_matrix;
            self.normals_matrix = self.model_matrix;
            self.should_update_matrices = false
        }
        (self.model_matrix, self.normals_matrix)
    }
    pub fn rotate(&mut self, radians: f32) {
        self.render_rot += radians;
        self.should_update_matrices = true
    }
    pub fn translate(&mut self, translation: &TVec3<f32>) {
        self.render_pos += translation;
        self.should_update_matrices = true
    }

    // Interpolates between the actual state of the ant (simulation), and the current
    // render state using the given interpolation ratio
    pub fn interpolate_state(&mut self, interpolation_ratio: f32) {
        let delta_p = Simulation::render_position(self.position) - self.render_pos;
        self.translate(&(delta_p * interpolation_ratio));

        let delta_r = self.direction.as_angle() - self.render_rot;
        self.rotate(delta_r * interpolation_ratio);
    }
}
