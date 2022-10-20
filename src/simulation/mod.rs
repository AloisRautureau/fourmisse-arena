pub mod ant;
pub mod map;
pub mod instruction;

use std::cell::RefCell;
use ant::Ant;
use map::Map;
use instruction::InstructionSet;
use crate::simulation::ant::Color;

// Contient l'état actuel d'une simulation
pub struct SimulationState {
    entities: Vec<RefCell<Ant>>,
    map: Map,
    instructions: [InstructionSet; 2]
}
impl SimulationState {
    pub fn new(size: (usize, usize)) -> Self {
        Self {
            entities: vec!(),
            map: Map::new_empty(size),
            instructions: [vec!(), vec!()]
        }
    }

    pub fn load_map(&mut self, path: &str) {
        let (map, entities) = Map::load_file(path);
        self.map = map;
        self.entities = entities;
    }

    // Passe au prochain état de la simulation
    pub fn process_tick(&mut self) {
        // Fait passer chaque fourmi au prochain tick
        for entity in &mut self.entities {
            let instruction_set_index = match entity.color {
                Color::Red => 0,
                Color::Black => 1
            };
            entity.process_tick(&mut self.map, &mut self.instructions[instruction_set_index])
        }

        // Puis nettoie les fourmis mortes s'il y en a
        let mut to_remove: Vec<usize> = vec!();
        for (i, entity) in self.entities.iter_mut().enumerate() {
            if self.map.is_surrounded(entity.position) {
                self.map.remove_ant(entity.position);
                to_remove.push(i);
            }
        }
        for i in to_remove {
            self.entities.remove(i);
        }
    }
}