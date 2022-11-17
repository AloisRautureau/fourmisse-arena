use rand::{Rng, thread_rng};
use rustc_hash::FxHashSet;
use crate::ecs::{EntityId, Position, CellType, Markers, Colour, FoodContainer, Direction, ExecutionContext};
use crate::query;
use crate::resources::{Cond, Instruction, SenseDirection, TurnDirection};
use crate::resources::Instruction::*;
use crate::simulation::Simulation;

impl Simulation {
    /// Given an entity, executes an instruction from this entity's perspective
    pub fn execute_instruction(&mut self, entity: EntityId, instruction: &Instruction) {
        let jump_label = match instruction {
            Sense(dir, if_label, else_label, cond) => {
                let position = self.entities.component::<Position>(entity).unwrap();
                let direction = self.entities.component::<Direction>(entity).unwrap();
                let target = match dir {
                    SenseDirection::Ahead => position.translate(direction),
                    SenseDirection::Left => position.translate(&direction.left()),
                    SenseDirection::Right => position.translate(&direction.right()),
                    SenseDirection::Here => *position
                };

                if self.check_cond(cond, self.entities.component::<Colour>(entity).unwrap(), &target) {
                    Some(if_label)
                } else {
                    Some(else_label)
                }
            },
            Mark(i) => {
                let current_position = *self.entities.component::<Position>(entity)
                    .unwrap();
                let colour = *self.entities.component::<Colour>(entity).unwrap();
                self.set_marker(&current_position, &colour, *i as usize);
                None
            },
            Unmark(i) => {
                let current_position = *self.entities.component::<Position>(entity)
                    .unwrap();
                let colour = *self.entities.component::<Colour>(entity).unwrap();
                self.unset_marker(&current_position, &colour, *i as usize);
                None
            },
            Pickup(fail_label) => {
                let current_position = self.entities.component::<Position>(entity)
                    .unwrap();
                if let Some(cell) =  self.cell_at_position(current_position) {
                    let can_pickup = {
                        let ant_food = self.entities.component::<FoodContainer>(entity).unwrap();
                        let cell_food = self.entities.component::<FoodContainer>(cell).unwrap();
                        ant_food.holding < ant_food.capacity && cell_food.holding != 0
                    };
                    if can_pickup {
                        self.entities.component_mut::<FoodContainer>(entity).unwrap().holding += 1;
                        self.entities.component_mut::<FoodContainer>(cell).unwrap().holding -= 1;
                        None
                    } else {
                        Some(fail_label)
                    }
                } else {
                    None
                }
            },
            Drop => {
                let current_position = self.entities.component::<Position>(entity)
                    .unwrap();
                if let Some(cell) = self.cell_at_position(current_position) {
                    if self.entities.component::<FoodContainer>(entity).unwrap().holding > 0 {
                        self.entities.component_mut::<FoodContainer>(entity).unwrap().holding -= 1;
                        self.entities.component_mut::<FoodContainer>(cell).unwrap().holding += 1;
                    }
                }
                None
            },
            Turn(dir) => {
                if dir == &TurnDirection::Left {
                    self.entities.component_mut::<Direction>(entity).unwrap().turn_left()
                } else {
                    self.entities.component_mut::<Direction>(entity).unwrap().turn_right()
                }
                None
            },
            Move(fail_label) => {
                let dir = self.entities.component::<Direction>(entity).unwrap();
                let target = self.entities.component::<Position>(entity).unwrap().translate(dir);
                if self.in_bounds(&target) && self.cell_is_empty(&target) {
                    *self.entities.component_mut::<Position>(entity).unwrap() = target;
                    self.entities.component_mut::<ExecutionContext>(entity).unwrap().cooldown = 14;

                    // Check for kills
                    if let Some(ant) = self.check_kill(&target) {
                        self.entities.delete_entity(ant)
                    } else {
                        for neighboring_pos in &self.neightboring_cells(&target) {
                            if let Some(ant) = self.check_kill(neighboring_pos) {
                                self.entities.delete_entity(ant)
                            }
                        }
                    }

                    None
                } else {
                    Some(fail_label)
                }
            },
            Flip(p, if_label, else_label) => {
                let mut rng = thread_rng();
                if rng.gen_range(0..*p) == 0 {
                    Some(if_label)
                } else {
                    Some(else_label)
                }
            },
            Goto(label) => Some(label),
            _ => None
        };

        if let Some(instr) = jump_label {
            self.entities.component_mut::<ExecutionContext>(entity).unwrap().current_instruction = *instr
        } else {
            self.entities.component_mut::<ExecutionContext>(entity).unwrap().current_instruction += 1
        }
    }

    fn cell_at_position(&self, position: &Position) -> Option<EntityId> {
        self.entities.query(&query!(&self.entities, Position, CellType, FoodContainer))
            .filter(|c| self.entities.component::<Position>(*c).unwrap() == position)
            .next()
    }

    fn in_bounds(&self, position: &Position) -> bool {
        position.x < self.map_width && position.y < self.map_height
    }

    fn set_marker(&mut self, position: &Position, colour: &Colour, index: usize) {
        if let Some(cell) = self.cell_at_position(position) {
            let markers = self.entities.component_mut::<Markers>(cell).unwrap();
            if colour == &Colour::Red {
                markers.red_markers |= (1 << index)
            } else {
                markers.black_markers |= (1 << index)
            }
        }
    }
    fn unset_marker(&mut self, position: &Position, colour: &Colour, index: usize) {
        if let Some(cell) = self.cell_at_position(position) {
            let markers = self.entities.component_mut::<Markers>(cell).unwrap();
            if colour == &Colour::Red {
                markers.red_markers &= !(1 << index)
            } else {
                markers.black_markers &= !(1 << index)
            }
        }
    }

    fn cell_is_empty(&self, position: &Position) -> bool {
        self.entities.query(&query!(&self.entities, Position, ExecutionContext))
            .find(|e| self.entities.component::<Position>(*e).unwrap() == position)
            .is_none()
        && self.entities.query(&query!(&self.entities, Position, CellType))
            .find(|e|
                self.entities.component::<CellType>(*e).unwrap() == &CellType::Obstacle
                    && self.entities.component::<Position>(*e).unwrap() == position
            )
            .is_none()
    }

    fn check_cond(&self, cond: &Cond, colour: &Colour, position: &Position) -> bool {
        match cond {
            Cond::Friend => {
                let mut ants = self.entities.query(&query!(&self.entities, Position, ExecutionContext));
                ants
                    .find(|a|
                        self.entities.component::<Position>(*a).unwrap() == position
                            && self.entities.component::<Colour>(*a).unwrap() == colour
                    )
                    .is_some()
            },
            Cond::Foe => {
                let mut ants = self.entities.query(&query!(&self.entities, Position, ExecutionContext));
                ants
                    .find(|a|
                        self.entities.component::<Position>(*a).unwrap() == position
                            && self.entities.component::<Colour>(*a).unwrap() == &colour.opposite()
                    )
                    .is_some()
            },
            Cond::FriendWithFood => {
                let mut ants = self.entities.query(&query!(&self.entities, Position, ExecutionContext));
                ants
                    .find(|a|
                        self.entities.component::<Position>(*a).unwrap() == position
                            && self.entities.component::<Colour>(*a).unwrap() == colour
                            && self.entities.component::<FoodContainer>(*a).unwrap().holding != 0
                    )
                    .is_some()
            },
            Cond::FoeWithFood => {
                let mut ants = self.entities.query(&query!(&self.entities, Position, ExecutionContext));
                ants
                    .find(|a|
                        self.entities.component::<Position>(*a).unwrap() == position
                            && self.entities.component::<Colour>(*a).unwrap() == &colour.opposite()
                            && self.entities.component::<FoodContainer>(*a).unwrap().holding != 0
                    )
                    .is_some()
            },
            Cond::Food => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, CellType, FoodContainer));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && self.entities.component::<FoodContainer>(*c).unwrap().holding != 0
                    )
                    .is_some()
            },
            Cond::Rock => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, CellType));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && self.entities.component::<CellType>(*c).unwrap() == &CellType::Obstacle
                    )
                    .is_some()
            },
            Cond::Marker(i) => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, Markers));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && (self.entities.component::<Markers>(*c).unwrap().get(colour) & (1 << i)) != 0
                    )
                    .is_some()
            },
            Cond::FoeMarker => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, Markers));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && self.entities.component::<Markers>(*c).unwrap().get(&colour.opposite()) != &0
                    )
                    .is_some()
            },
            Cond::Home => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, CellType, Colour));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && self.entities.component::<Colour>(*c).unwrap() == colour
                    )
                    .is_some()
            },
            Cond::FoeHome => {
                let mut cells = self.entities.query(&query!(&self.entities, Position, CellType, Colour));
                cells
                    .find(|c|
                        self.entities.component::<Position>(*c).unwrap() == position
                            && self.entities.component::<Colour>(*c).unwrap() == &colour.opposite()
                    )
                    .is_some()
            }
        }
    }

    fn check_kill(&self, position: &Position) -> Option<EntityId> {
        let ants = self.entities.query(&query!(&self.entities, Position, ExecutionContext, Colour));
        let neighboring_cells = self.neightboring_cells(position);

        let mut red_ants = 0;
        let mut black_ants = 0;
        let mut self_colour = None;
        let mut entity = None;
        for ant in ants {
            let current_position = self.entities.component::<Position>(ant).unwrap();
            let colour = self.entities.component::<Colour>(ant).unwrap();
            if neighboring_cells.contains(&current_position) {
                if colour == &Colour::Red { red_ants += 1 } else { black_ants += 1 }
            } else if current_position == position {
                self_colour = Some(colour);
                entity = Some(ant)
            }
        }
        match self_colour {
            Some(&Colour::Red) if black_ants >= 5 => entity,
            Some(&Colour::Black) if red_ants >= 5 => entity,
            _ => None,
        }
    }
    fn neightboring_cells(&self, position: &Position) -> FxHashSet<Position> {
        Direction::iter()
            .flat_map(|dir| {
                let target = position.translate(&dir);
                if self.in_bounds(&target) {
                    Some(target)
                } else {
                    None
                }
            })
            .collect()
    }
}

/*
use super::instruction::{Instruction, Instruction::*, InstructionSet};
use super::map::Map;
use crate::rendering_engine::{
    LightSource, Material, RenderingEngine, ResourceHandle, ResourceHandler,
};
use crate::simulation::instruction::{SenseDirection, TurnDirection};
use crate::simulation::map::AntRef;
use crate::simulation::{Simulation, HEXAGON_HEIGHT, HEXAGON_RADIUS, HEXAGON_WIDTH};
use nalgebra_glm::{
    identity, inverse_transpose, make_vec3, pi, rotate_normalized_axis, translate, vec3,
    vec3_to_vec4, TMat4, TVec3,
};
use rand::Rng;
use std::fmt::Debug;
use std::rc::Rc;

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
                shininess: 128.0,
                specular_intensity: 1.0,
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
            resource_handler
                .models
                .fetch_model_vertices(&ant_model_handle),
            self.model_matrices(),
            &self.material,
        );

        if self.has_food {
            Simulation::render_food_piece(
                renderer,
                food_model_handle,
                resource_handler,
                self.render_pos + vec3(0f32, 0.3, 0f32),
                self.render_rot,
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
 */