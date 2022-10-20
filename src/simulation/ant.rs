use rand::Rng;
use crate::simulation::instruction::{SenseDirection, TurnDirection};
use super::instruction::{InstructionSet, Instruction, Instruction::*};
use super::map::Map;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Color {
    Red,
    Black
}
impl Default for Color {
    fn default() -> Self { Self::Red }
}
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum CardinalDirection {
    North,
    South,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast
}
impl Default for CardinalDirection {
    fn default() -> Self { Self::North }
}
impl CardinalDirection {
    pub fn next(self) -> Self {
        match self {
            Self::North => Self::NorthEast,
            Self::NorthEast => Self::SouthEast,
            Self::SouthEast => Self::South,
            Self::South => Self::SouthWest,
            Self::SouthWest => Self::NorthWest,
            Self::NorthWest => Self::North
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::North => Self::NorthWest,
            Self::NorthEast => Self::North,
            Self::SouthEast => Self::NorthEast,
            Self::South => Self::SouthEast,
            Self::SouthWest => Self::South,
            Self::NorthWest => Self::SouthWest
        }
    }
}

// Completely represents one ant
#[derive(Debug)]
pub struct Ant {
    pub color: Color,
    pub position: (usize, usize),
    current_instruction: usize,
    cooldown: usize,
    direction: CardinalDirection,

    has_food: bool
}
impl Ant {
    // Créé une nouvelle fourmi de la couleur et à la position souhaitée
    pub fn new(color: Color, position: (usize, usize)) -> Self {
        println!("created a new ant, yay!");
        Self {
            color,
            position,
            current_instruction: 0,
            cooldown: 0,
            direction: CardinalDirection::default(),
            has_food: false
        }
    }

    // Fait avancer la fourmi d'un tick
    // Elle exécute son instruction actuelle, modifie son compteur d'instruction,
    // décremente son compteur de repos
    pub fn process_tick(&mut self, map: &mut Map, instructions: &InstructionSet) {
        // Exécute l'instruction courante si on est hors cooldown
        if self.cooldown == 0 {
            let goto_instruction = self.exec(instructions[self.current_instruction], map);

            // Modifie son compteur en accord avec l'instruction précedente
            if let Some(instruction) = goto_instruction {
                self.current_instruction = instruction;
            } else {
                self.current_instruction += 1;
            }
        }
        // Décrémente son compteur de repos
        self.cooldown -= 1;
    }

    // Exécute une instruction donnée selon l'état de la fourmi et de la carte
    // Renvoie un numéro d'instruction en cas de jump
    fn exec(&mut self, instruction: Instruction, map: &mut Map) -> Option<usize> {
        match instruction {
            Sense(dir, true_label, false_label, cond) => {
                // Calcule la cellule à vérifier
                let cell = match dir {
                    SenseDirection::Ahead => self.target_cell(),
                    SenseDirection::Left => {
                        self.direction = self.direction.previous();
                        let c = self.target_cell();
                        self.direction = self.direction.next();
                        c
                    }
                    SenseDirection::Right => {
                        self.direction = self.direction.next();
                        let c = self.target_cell();
                        self.direction = self.direction.previous();
                        c
                    }
                    SenseDirection::Here => self.position
                };
                // Puis vérifie la condition donnée
                if map.check_cond(cell, self.color, cond) {
                    Some(true_label)
                } else {
                    Some(false_label)
                }
            },
            Mark(i) => {
                map.mark_pheromone(self.position, i, self.color);
                None
            },
            Unmark(i) => {
                map.unmark_pheromone(self.position, i, self.color);
                None
            },
            Pickup(fail_label) => {
                if !self.has_food && map.pickup_food(self.position) {
                    self.has_food = true;
                    None
                } else {
                    Some(fail_label)
                }
            },
            Drop => {
                if self.has_food {
                    map.drop_food(self.position);
                }
                None
            },
            Turn(dir) => {
                if dir == TurnDirection::Left {
                    self.direction = self.direction.previous()
                } else {
                    self.direction = self.direction.next()
                }
                None
            },
            Move(fail_label) => {
                if map.try_move(self.position, self.target_cell()) {
                    self.position = self.target_cell();
                    self.cooldown = 14;
                    None
                } else {
                    Some(fail_label)
                }
            },
            Flip(p, success_label, failure_label) => {
                let mut rng = rand::prelude::thread_rng();
                if rng.gen_range(0..p) == 0 {
                    Some(success_label)
                } else {
                    Some(failure_label)
                }
            },
            Goto(label) => Some(label)
        }
    }

    fn target_cell(&self) -> (usize, usize) {
        let (x, y) = self.position;
        match self.direction {
            CardinalDirection::North => (x, y+1),
            CardinalDirection::NorthEast => (x+1, y+1),
            CardinalDirection::NorthWest => (x-1, y+1),
            CardinalDirection::South => (x, y-1),
            CardinalDirection::SouthEast => (x+1, y-1),
            CardinalDirection::SouthWest => (x-1, y-1)
        }
    }
}