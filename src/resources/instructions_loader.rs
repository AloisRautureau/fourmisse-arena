use crate::resources::ResourceLoader;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Labels are used to implement goto-like instructions, as an index to jump to in
/// the instruction set
type Label = usize;

/// Directions that can be sensed
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SenseDirection {
    Ahead,
    Left,
    Right,
    Here,
}
impl From<String> for SenseDirection {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Ahead" => Self::Ahead,
            "LeftAhead" => Self::Left,
            "RightAhead" => Self::Right,
            "Here" => Self::Here,
            _ => panic!("Not a valid SenseDirection"),
        }
    }
}

/// Directions that an ant can turn in
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TurnDirection {
    Left,
    Right,
}
impl From<String> for TurnDirection {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Left" => Self::Left,
            "Right" => Self::Right,
            _ => panic!("Not a valid TurnDirection"),
        }
    }
}

/// Describes conditions that ants can check on neighboring cells
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Cond {
    Friend,
    Foe,
    FriendWithFood,
    FoeWithFood,
    Food,
    Rock,
    Marker(u8),
    FoeMarker,
    Home,
    FoeHome,
}
impl From<(String, Option<u8>)> for Cond {
    fn from((s, i): (String, Option<u8>)) -> Self {
        let mut instruction_parts = s.split(' ');
        match instruction_parts.next().unwrap() {
            "Friend" => Self::Friend,
            "Foe" => Self::Foe,
            "FriendWithFood" => Self::FriendWithFood,
            "FoeWithFood" => Self::FoeWithFood,
            "Food" => Self::Food,
            "Rock" => Self::Rock,
            "Marker" => Self::Marker(i.expect("Missing argument on Marker condition")),
            "FoeMarker" => Self::FoeMarker,
            "Home" => Self::Home,
            "FoeHome" => Self::FoeHome,
            _ => panic!("Not a valid TurnDirection"),
        }
    }
}

/// Complete instruction set as described in the project description
#[derive(Debug, Copy, Clone)]
pub enum Instruction {
    Sense(SenseDirection, Label, Label, Cond),
    Mark(u8),
    Unmark(u8),
    Pickup(Label),
    Drop,
    Turn(TurnDirection),
    Move(Label),
    Flip(u8, Label, Label),
    Goto(Label),

    // This instruction is used to replace invalid instructions that are parsed (Mark/Unmark with i > 5 for example)
    Dummy
}
impl From<(String, &HashMap<String, usize>)> for Instruction {
    fn from((instr, label_map): (String, &HashMap<String, usize>)) -> Self {
        let trimmed = instr.trim();
        let mut instruction_parts = trimmed.split(' ');
        let instruction_type = instruction_parts.next().unwrap();
        match instruction_type {
            "Sense" => {
                let direction = SenseDirection::from(String::from(
                    instruction_parts
                        .next()
                        .expect("Missing parameters to Sense instruction"),
                ));
                let label1 = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Sense instruction"),
                    )
                    .expect("Use of an undefined label in Sense instruction");
                let label2 = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Sense instruction"),
                    )
                    .expect("Use of an undefined label in Sense instruction");
                let cond = Cond::from((
                    String::from(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Sense instruction"),
                    ),
                    instruction_parts
                        .next()
                        .and_then(|x| x.parse::<u8>().ok()),
                ));
                Instruction::Sense(direction, *label1, *label2, cond)
            }
            "Mark" => {
                let i = instruction_parts
                    .next()
                    .expect("Missing argument on Mark instruction")
                    .parse::<u8>()
                    .expect("Argument on Mark instruction is not an integer");
                if i < 6 {
                    Instruction::Mark(i)
                } else {
                    Instruction::Dummy
                }
            }
            "Unmark" => {
                let i = instruction_parts
                    .next()
                    .expect("Missing argument on Unmark instruction")
                    .parse::<u8>()
                    .expect("Argument on Unmark instruction is not an integer");
                if i < 6 {
                    Instruction::Unmark(i)
                } else {
                    Instruction::Dummy
                }
            }
            "PickUp" => {
                let label = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Pickup instruction"),
                    )
                    .expect("Use of an undefined label in Pickup instruction");
                Instruction::Pickup(*label)
            }
            "Drop" => Instruction::Drop,
            "Turn" => {
                let dir = TurnDirection::from(String::from(
                    instruction_parts
                        .next()
                        .expect("Missing argument on Turn instruction"),
                ));
                Instruction::Turn(dir)
            }
            "Move" => {
                let label = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Move instruction"),
                    )
                    .expect("Use of an undefined label in Move instruction");
                Instruction::Move(*label)
            }
            "Flip" => {
                let p = instruction_parts
                    .next()
                    .expect("Missing argument on Flip instruction")
                    .parse::<u8>()
                    .expect("Argument of Flip instruction is not an integer");
                let label1 = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Flip instruction"),
                    )
                    .expect("Use of an undefined label in Flip instruction");
                let label2 = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Flip instruction"),
                    )
                    .expect("Use of an undefined label in Flip instruction");
                Instruction::Flip(p, *label1, *label2)
            }
            "Goto" => {
                let label_name = instruction_parts.next().expect("Missing argument on Goto instruction");
                let label = label_map
                    .get(
                        label_name
                    )
                    .unwrap_or_else(|| panic!("Use of undefined label {} in Goto instruction", label_name));
                Instruction::Goto(*label)
            }
            _ => panic!("{} is not a valid instruction", instr),
        }
    }
}

/// Stores a succession of instructions
pub type InstructionSet = Vec<Instruction>;

/// Loads an instruction set from a .brain file on disk
pub struct InstructionsLoader {
    pub path: String,
    label_map: HashMap<String, usize>
}
impl InstructionsLoader {
    /// Initializes a new instruction loader for the given file
    pub fn new(path: &str) -> InstructionsLoader {
        InstructionsLoader {
            path: String::from(path),
            label_map: HashMap::default()
        }
    }
}
impl ResourceLoader for InstructionsLoader {
    type Item = InstructionSet;

    fn load(&mut self) -> Self::Item {
        fn read_lines<P>(filename: P) -> io::Result<io::Lines<BufReader<File>>>
            where
                P: AsRef<Path>,
        {
            let file = File::open(filename)?;
            Ok(BufReader::new(file).lines())
        }

        let lines = read_lines(&self.path)
            .expect("Could not read the given .brain file")
            .filter_map(|l| {
                if let Ok(s) = l {
                    let s = s.trim();
                    if s.is_empty() {
                        None
                    } else {
                        Some(String::from(s))
                    }
                } else {
                    None
                }
            })
            .enumerate();
        let instruction_regex =
            Regex::new(r"Sense|Drop|Mark|Unmark|PickUp|Turn|Move|Flip|Goto").unwrap();

        // During a first pass, we simply care about the labels
        // This lets us create a Map of (label -> line) to make the implementation
        // way more efficient
        for (i, line) in lines {
            // The line is either an instruction or a label
            if !instruction_regex.is_match(&line) {
                let label = line.split(':').next().unwrap();
                // Little manipulation so that the label is mapped to its location
                // if the labels before it did not exist
                // This lets us completely ignore labels later on, caring only about
                // the instruction's index
                self.label_map.insert(String::from(label), i - self.label_map.len());
            }
        }
        // We can then do a second pass, this time taking care of the
        // actual instructions
        let lines = read_lines(&self.path).expect("Could not read the given .brain file");
        let mut instructions: InstructionSet = vec![];
        for line in lines.flatten() {
            // The line is either an instruction or a label
            if instruction_regex.is_match(&line) {
                instructions.push(Instruction::from((line, &self.label_map)));
            }
        }

        instructions
    }
}