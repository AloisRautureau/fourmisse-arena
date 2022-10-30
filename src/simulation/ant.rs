use std::fmt::{Debug};
use std::rc::Rc;
use rand::Rng;
use crate::simulation::instruction::{SenseDirection, TurnDirection};
use crate::simulation::map::AntRef;
use super::instruction::{InstructionSet, Instruction, Instruction::*};
use super::map::Map;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Colour {
    Red,
    Black
}
impl Colour {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Red => Self::Black,
            _ => Self::Red
        }
    }
    pub fn as_index(&self) -> usize {
        match self {
            Self::Red => 0,
            _ => 1
        }
    }
}
impl Default for Colour {
    fn default() -> Self { Self::Red }
}
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum CardinalDirection {
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast
}
impl Default for CardinalDirection {
    fn default() -> Self { Self::East }
}
impl CardinalDirection {
    pub fn right(self) -> Self {
        match self {
            Self::West => Self::NorthWest,
            Self::NorthWest => Self::NorthEast,
            Self::NorthEast => Self::East,
            Self::East => Self::SouthEast,
            Self::SouthEast => Self::SouthWest,
            Self::SouthWest => Self::West
        }
    }

    pub fn left(self) -> Self {
        match self {
            Self::West => Self::SouthWest,
            Self::SouthWest => Self::SouthEast,
            Self::SouthEast => Self::East,
            Self::East => Self::NorthEast,
            Self::NorthEast => Self::NorthWest,
            Self::NorthWest => Self::West
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

    current_instruction: usize,
    cooldown: usize,
    direction: CardinalDirection
}
impl Ant {
    // Creates a new ant of the given colour
    pub fn new(id: usize, colour: Colour, position: (usize, usize)) -> Self {
        Self {
            id,
            colour,
            position,
            has_food: false,

            current_instruction: 0,
            cooldown: 0,
            direction: CardinalDirection::default(),
        }
    }

    // Processes one tick, executing a command if the ant is off cooldown, and
    // reducing said cooldown by 1
    pub fn process_tick(ant: AntRef, map: &mut Map, instructions: &InstructionSet) {
        if (*ant).borrow().cooldown == 0 {
            let current_instruction = instructions.get((*ant).borrow().current_instruction)
                .expect("Instruction count is out of bounds");
            Self::exec(Rc::clone(&ant), current_instruction, map);
        } else {
            (*ant).borrow_mut().cooldown -= 1
        }
    }

    // Executes a given instruction, ant's state and map
    // The instruction can change the ant's state
    // Returns the index of the next instruction
    fn exec(ant: AntRef, instruction: &Instruction, map: &mut Map) -> () {
         let jump_instruction = match *instruction {
            Sense(dir, true_label, false_label, cond) => {
                // Calculates the target cell's index
                let cell = (*ant).borrow().target_cell(dir);
                // Then checks the given condition and change the current instruction
                // accordingly
                Some(if map.check_condition(cond, (*ant).borrow().colour, cell) {
                    true_label
                } else {
                    false_label
                })
            },
            Mark(i) => {
                map.mark_pheromone((*ant).borrow().position, i, (*ant).borrow().colour);
                None
            },
            Unmark(i) => {
                map.unmark_pheromone((*ant).borrow().position, i, (*ant).borrow().colour);
                None
            },
            Pickup(fail_label) => {
                if !(*ant).borrow().has_food && map.pickup_food((*ant).borrow().position) {
                    (*ant).borrow_mut().has_food = true;
                    None
                } else {
                    Some(fail_label)
                }
            },
            Drop => {
                if (*ant).borrow().has_food {
                    map.drop_food((*ant).borrow().position);
                }
                (*ant).borrow_mut().has_food = false;
                None
            },
            Turn(TurnDirection::Left) => {
                let next_direction = (*ant).borrow().direction.left();
                (*ant).borrow_mut().direction = next_direction;
                None
            },
            Turn(TurnDirection::Right) => {
                let next_direction = (*ant).borrow().direction.right();
                (*ant).borrow_mut().direction = next_direction;
                None
            },
            Move(fail_label) => {
                let from = (*ant).borrow().position;
                let to = (*ant).borrow().target_cell(SenseDirection::Ahead);
                if map.move_to(from, to) {
                    (*ant).borrow_mut().position = to;
                    (*ant).borrow_mut().cooldown = 14;
                    None
                } else {
                    Some(fail_label)
                }
            },
            Flip(p, success_label, failure_label) => {
                let rng = rand::thread_rng().gen_range(0..p);
                Some(if rng == 0 {
                    success_label
                } else {
                    failure_label
                })
            },
            Goto(label) => Some(label)
        };

        if let Some(instruction) = jump_instruction {
            (*ant).borrow_mut().current_instruction = instruction
        } else {
            (*ant).borrow_mut().current_instruction += 1
        }
    }

    fn target_cell(&self, direction: SenseDirection) -> (usize, usize) {
        let (x, y) = self.position;
        let sense_direction = match direction {
            SenseDirection::Right => self.direction.right(),
            SenseDirection::Left => self.direction.left(),
            SenseDirection::Here => return self.position,
            _ => self.direction
        };
        match sense_direction {
            CardinalDirection::West => (x-1, y),
            CardinalDirection::NorthEast => (x+1, y-1),
            CardinalDirection::NorthWest => (x-1, y-1),
            CardinalDirection::East => (x+1, y),
            CardinalDirection::SouthEast => (x+1, y+1),
            CardinalDirection::SouthWest => (x-1, y+1)
        }
    }
}