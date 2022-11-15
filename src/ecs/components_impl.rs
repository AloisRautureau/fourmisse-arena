use nalgebra_glm::pi;
use crate::resources::ResourceId;

/// Defines a position over a 2D grid
pub struct Position {
    pub x: usize,
    pub y: usize
}

/// Entities with this component can hold food up to a maximum capacity
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

/// All the types a cell can have as labels
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