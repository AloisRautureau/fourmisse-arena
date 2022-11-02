pub mod ant;
pub mod instruction;
pub mod map;

use crate::simulation::ant::Ant;
use crate::simulation::instruction::load_instructionset;
use crate::simulation::map::AntRef;
use instruction::InstructionSet;
use map::Map;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;

// Represents the current state of a simulation
pub struct Simulation {
    pub ants: Vec<AntRef>,
    pub map: Map,
    instructions: [InstructionSet; 2],
}
impl Simulation {
    pub fn new(map_path: &str, red_brain_path: &str, black_brain_path: &str) -> Self {
        let (map, ants) = Map::load_file(map_path);
        Self {
            ants,
            map,
            instructions: [
                load_instructionset(red_brain_path),
                load_instructionset(black_brain_path),
            ],
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
}
