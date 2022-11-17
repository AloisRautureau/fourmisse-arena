use nalgebra_glm::pi;
use crate::resources::ResourceId;

/// Defines a position over a 2D grid
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize
}
impl Position {
    pub fn translate(&self, dir: &Direction) -> Position {
        let (x, y) = match dir {
            Direction::East => (self.x + 1, self.y),
            Direction::SouthEast => (self.x + 1, self.y + 1),
            Direction::SouthWest => (self.x - 1, self.y + 1),
            Direction::West => (self.x - 1, self.y),
            Direction::NorthWest => (self.x - 1, self.y - 1),
            Direction::NorthEast => (self.x + 1, self.y - 1)
        };
        Position { x, y }
    }
}

/// Entities with this component can hold food up to a maximum capacity
#[derive(Debug)]
pub struct FoodContainer {
    pub holding: u32,
    pub capacity: u32
}

/// Defines markers (only used for cells as of now)
#[derive(Default)]
pub struct Markers {
    pub red_markers: u8,
    pub black_markers: u8
}
impl Markers {
    pub fn get(&self, colour: &Colour) -> &u8 {
        if colour == &Colour::Red {
            &self.red_markers
        } else {
            &self.black_markers
        }
    }
    pub fn get_mut(&mut self, colour: &Colour) -> &mut u8 {
        if colour == &Colour::Red {
            &mut self.red_markers
        } else {
            &mut self.black_markers
        }
    }
}

/// All the types a cell can have as labels
#[derive(PartialEq, Eq)]
pub enum CellType {
    Empty,
    Obstacle,
    Nest
}

/// Everything required to execute ant code
pub struct ExecutionContext {
    pub current_instruction: usize,
    pub instruction_set_id: ResourceId,
    pub cooldown: u8
}

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
pub enum Direction {
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}
impl Direction {
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

    pub fn turn_right(&mut self) {
        *self = self.right()
    }
    pub fn turn_left(&mut self) {
        *self = self.left()
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

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::East, Self::SouthEast, Self::SouthWest, Self::West, Self::NorthWest, Self::NorthEast]
            .into_iter()
    }
}