type Label = usize;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SenseDirection {
    Ahead,
    Left,
    Right,
    Here
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TurnDirection {
    Left,
    Right
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
    FoeHome
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
    Goto(Label)
}

pub type InstructionSet = Vec<Instruction>;