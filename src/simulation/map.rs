use super::ant::{Ant, Colour};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};

use crate::simulation::instruction::Cond;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut};
use std::rc::Rc;

pub type AntRef = Rc<RefCell<Ant>>;

pub enum Cell {
    Empty {
        food: u8,
        occupant: Option<AntRef>,
        markers: [u8; 2],
    },
    Obstacle,
    Nest {
        colour: Colour,
        food: u8,
        occupant: Option<AntRef>,
        markers: [u8; 2],
    },
}

// A map contains a matrix of cells, which can be obstacles or empty.
// Empty cells can have at most 9 units of food on them
pub struct Map {
    cells: Vec<Cell>,
    size: (usize, usize),
}
impl Map {
    // Loads a map from a file
    // Returns loaded map, as well as a vector of ants derived from it
    pub fn load_file(path: &str) -> (Self, Vec<AntRef>) {
        let mut ants = vec![];
        let mut map = Self {
            cells: Vec::new(),
            size: (0, 0),
        };

        let mut f = BufReader::new(File::open(path).expect("could not open file"));
        let mut buff = Vec::<u8>::new();

        // First read the header
        f.read_until(b'\n', &mut buff)
            .expect("could not read from file");
        buff.clear();
        // x size
        f.read_until(b'\n', &mut buff)
            .expect("could not read from file");
        let s = String::from_utf8(buff).expect("invalid characters in instruction file");
        map.size.0 = s
            .trim()
            .parse::<usize>()
            .expect("Size x in header is not an integer");
        buff = s.into_bytes();
        buff.clear();

        // y size
        f.read_until(b'\n', &mut buff)
            .expect("could not read from file");
        let s = String::from_utf8(buff).expect("invalid characters in instruction file");
        map.size.1 = s
            .trim()
            .parse::<usize>()
            .expect("Size y in header is not an integer");
        buff = s.into_bytes();
        buff.clear();

        // And now the actual map
        let mut x = 0;
        let mut y = 0;
        let mut id = 0;
        let mut get_id = || {
            id += 1;
            id - 1
        };
        while f
            .read_until(b'\0', &mut buff)
            .expect("could not read from file")
            != 0
        {
            let s = String::from_utf8(buff).expect("invalid characters in instruction file");

            for c in s.chars() {
                match c {
                    '#' => map.cells.push(Cell::Obstacle),
                    '.' => map.cells.push(Cell::Empty {
                        food: 0,
                        occupant: None,
                        markers: [0; 2],
                    }),
                    '+' => {
                        let new_ant = Ant::new(get_id(), Colour::Red, (x, y));
                        let ant_ref = Rc::new(RefCell::new(new_ant));
                        ants.push(Rc::clone(&ant_ref));
                        map.cells.push(Cell::Nest {
                            colour: Colour::Red,
                            food: 0,
                            occupant: Some(Rc::clone(&ant_ref)),
                            markers: [0; 2],
                        });
                    }
                    '-' => {
                        let new_ant = Ant::new(get_id(), Colour::Black, (x, y));
                        let ant_ref = Rc::new(RefCell::new(new_ant));
                        ants.push(Rc::clone(&ant_ref));
                        map.cells.push(Cell::Nest {
                            colour: Colour::Black,
                            food: 0,
                            occupant: Some(Rc::clone(&ant_ref)),
                            markers: [0; 2],
                        });
                    }
                    ' ' => (),
                    '\n' => {
                        y += 1;
                        x = 0;
                    }
                    _ => {
                        if c.is_digit(10) {
                            let food = c.to_digit(10).unwrap() as u8;
                            map.cells.push(Cell::Empty {
                                food,
                                occupant: None,
                                markers: [0; 2],
                            });
                        }
                    }
                }
                if !(c == ' ' || c == '\n') {
                    x += 1
                }
            }

            buff = s.into_bytes();
            buff.clear();
        }

        (map, ants)
    }

    pub fn mark_pheromone(&mut self, cell: (usize, usize), i: usize, color: Colour) {
        if i < 7 {
            match &mut self[cell] {
                Cell::Empty { markers, .. } => markers[color.as_index()] |= 1 << i,
                Cell::Nest { markers, .. } => markers[color.as_index()] |= 1 << i,
                _ => (),
            }
        }
    }
    pub fn unmark_pheromone(&mut self, cell: (usize, usize), i: usize, color: Colour) {
        if i < 7 {
            match &mut self[cell] {
                Cell::Empty { markers, .. } => markers[color.as_index()] &= !(1 << i),
                Cell::Nest { markers, .. } => markers[color.as_index()] |= !(1 << i),
                _ => (),
            }
        }
    }

    pub fn pickup_food(&mut self, cell: (usize, usize)) -> bool {
        match &mut self[cell] {
            Cell::Empty { food, .. } | Cell::Nest { food, .. } if *food > 0 => {
                *food -= 1;
                true
            }
            _ => false,
        }
    }
    pub fn drop_food(&mut self, cell: (usize, usize)) {
        match &mut self[cell] {
            Cell::Empty { food, .. } | Cell::Nest { food, .. } => *food += 1,
            _ => (),
        }
    }

    pub fn move_to(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if self.occupied(to) {
            false
        } else {
            let ant = match &mut self[from] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. }
                    if occupant.is_some() =>
                {
                    occupant.take()
                }
                _ => panic!("Tried to move from an obstacle or empty cell"),
            };
            match &mut self[to] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => *occupant = ant,
                _ => (),
            }
            true
        }
    }
    fn occupied(&self, cell: (usize, usize)) -> bool {
        // Checks whether what we want to check is in bounds or not
        if cell.0 >= self.size.0 || cell.1 >= self.size.1 {
            return true;
        }
        match &self[cell] {
            Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => occupant.is_some(),
            _ => true,
        }
    }

    pub fn check_condition(
        &self,
        condition: Cond,
        perspective: Colour,
        cell: (usize, usize),
    ) -> bool {
        // Checks whether what we want to check is in bounds or not
        if cell.0 >= self.size.0 || cell.1 >= self.size.1 {
            return false;
        }
        match condition {
            Cond::Friend => match &self[cell] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => {
                    if let Some(ref ant) = &occupant {
                        let ant: &RefCell<Ant> = ant.borrow();
                        ant.borrow().colour == perspective
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Cond::Foe => match &self[cell] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => {
                    if let Some(ant) = occupant {
                        let ant: &RefCell<Ant> = ant.borrow();
                        ant.borrow().colour != perspective
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Cond::FriendWithFood => match &self[cell] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => {
                    if let Some(ant) = occupant {
                        let ant: &RefCell<Ant> = ant.borrow();
                        ant.borrow().colour == perspective && ant.borrow().has_food
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Cond::FoeWithFood => match &self[cell] {
                Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } => {
                    if let Some(ant) = occupant {
                        let ant: &RefCell<Ant> = ant.borrow();
                        ant.borrow().colour != perspective && ant.borrow().has_food
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Cond::Food => match self[cell] {
                Cell::Empty { food, .. } | Cell::Nest { food, .. } => food != 0,
                _ => false,
            },
            Cond::Rock => matches!(self[cell], Cell::Obstacle),
            Cond::Marker(i) => match self[cell] {
                Cell::Empty { markers, .. } | Cell::Nest { markers, .. } => {
                    markers[perspective.as_index()] & (1 << i) != 0
                }
                _ => false,
            },
            Cond::FoeMarker => match self[cell] {
                Cell::Empty { markers, .. } | Cell::Nest { markers, .. } => {
                    markers[perspective.opposite().as_index()] != 0
                }
                _ => false,
            },
            Cond::Home => match self[cell] {
                Cell::Nest { colour, .. } => colour == perspective,
                _ => false,
            },
            Cond::FoeHome => match self[cell] {
                Cell::Nest { colour, .. } => colour != perspective,
                _ => false,
            },
        }
    }

    // Counts the total food count in both sides' nests
    pub fn points(&self) -> (u32, u32) {
        let (mut red_points, mut black_points) = (0, 0);
        for c in &self.cells {
            match c {
                Cell::Nest {
                    colour: Colour::Red,
                    food,
                    ..
                } => red_points += *food as u32,
                Cell::Nest {
                    colour: Colour::Black,
                    food,
                    ..
                } => black_points += *food as u32,
                _ => (),
            }
        }
        (red_points, black_points)
    }
}
impl Index<(usize, usize)> for Map {
    type Output = Cell;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (x, y) = index;
        let (size_x, size_y) = self.size;
        assert!(x < size_x && y < size_y);
        &self.cells[y * size_x + x]
    }
}
impl IndexMut<(usize, usize)> for Map {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (x, y) = index;
        let (size_x, size_y) = self.size;
        assert!(x < size_x && y < size_y);
        &mut self.cells[y * size_x + x]
    }
}
impl Debug for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, c) in self.cells.iter().enumerate() {
            if i % self.size.0 == 0 {
                if ((i / self.size.0) % 2) == 0 {
                    write!(f, "\n")?
                } else {
                    write!(f, "\n ")?
                }
            }
            write!(
                f,
                "{} ",
                match c {
                    Cell::Nest {
                        occupant: Some(ant),
                        ..
                    } => {
                        let ant: &RefCell<Ant> = ant.borrow();
                        if ant.borrow().colour == Colour::Black {
                            String::from("b")
                        } else {
                            String::from("r")
                        }
                    }
                    Cell::Nest {
                        colour: Colour::Red,
                        ..
                    } => String::from("+"),
                    Cell::Nest {
                        colour: Colour::Black,
                        ..
                    } => String::from("-"),
                    Cell::Empty {
                        occupant: Some(ant),
                        ..
                    } => {
                        let ant: &RefCell<Ant> = ant.borrow();
                        if ant.borrow().colour == Colour::Black {
                            String::from("b")
                        } else {
                            String::from("r")
                        }
                    }
                    Cell::Empty { food: 0, .. } => String::from("."),
                    Cell::Empty { food, .. } => food.to_string(),
                    Cell::Obstacle => String::from("#"),
                }
            )?
        }
        write!(f, "")
    }
}
