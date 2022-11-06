use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

type Label = usize;

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

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Cond {
    Friend,
    Foe,
    FriendWithFood,
    FoeWithFood,
    Food,
    Rock,
    Marker(usize),
    FoeMarker,
    Home,
    FoeHome,
}
impl From<(String, Option<usize>)> for Cond {
    fn from((s, i): (String, Option<usize>)) -> Self {
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

// Le set d'instructions fourni dans le pdf du projet
#[derive(Debug, Copy, Clone)]
pub enum Instruction {
    Sense(SenseDirection, Label, Label, Cond),
    Mark(usize),
    Unmark(usize),
    Pickup(Label),
    Drop,
    Turn(TurnDirection),
    Move(Label),
    Flip(usize, Label, Label),
    Goto(Label),
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
                        .and_then(|x| x.parse::<usize>().ok()),
                ));
                Instruction::Sense(direction, *label1, *label2, cond)
            }
            "Mark" => {
                let i = instruction_parts
                    .next()
                    .expect("Missing argument on Mark instruction")
                    .parse::<usize>()
                    .expect("Argument on Mark instruction is not an integer");
                Instruction::Mark(i)
            }
            "Unmark" => {
                let i = instruction_parts
                    .next()
                    .expect("Missing argument on Unmark instruction")
                    .parse::<usize>()
                    .expect("Argument on Unmark instruction is not an integer");
                Instruction::Unmark(i)
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
                    .parse::<usize>()
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
                let label = label_map
                    .get(
                        instruction_parts
                            .next()
                            .expect("Missing argument on Goto instruction"),
                    )
                    .expect("Use of an undefined label in Goto instruction");
                Instruction::Goto(*label)
            }
            _ => panic!("Invalid instruction"),
        }
    }
}

pub type InstructionSet = Vec<Instruction>;

pub fn load_instructionset(path: &str) -> InstructionSet {
    fn read_lines<P>(filename: P) -> io::Result<io::Lines<BufReader<File>>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        Ok(BufReader::new(file).lines())
    }

    let lines = read_lines(path)
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
    let mut labels_map: HashMap<String, usize> = HashMap::new();
    let mut offset = 0;
    for (i, line) in lines {
        // The line is either an instruction or a label
        if !instruction_regex.is_match(&line) {
            let label = line.split(':').next().unwrap();
            // Little manipulation so that the label is mapped to its location
            // if the labels before it did not exist
            // This lets us completely ignore labels later on, caring only about
            // the instruction's index
            labels_map.insert(String::from(label), i - offset);
            offset += 1;
        }
    }
    // We can then do a second pass, this time taking care of the
    // actual instructions
    let lines = read_lines(path).expect("Could not read the given .brain file");
    let mut instructions: InstructionSet = vec![];
    for line in lines.flatten() {
        // The line is either an instruction or a label
        if instruction_regex.is_match(&line) {
            instructions.push(Instruction::from((line, &labels_map)));
        }
    }

    instructions
}
