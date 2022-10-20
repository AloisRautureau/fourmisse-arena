use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use super::ant::{Color, Ant};
use super::instruction::{Cond, Cond::*};

use std::mem;
use std::fs;

pub enum WorldObject {
    RedAnt,
    BlackAnt,
    Rock,
    RedNest,
    BlackNest
}

// A map contains a matrix of cells, which can be obstacles or empty.
// Empty cells can have at most 9 units of food on them
pub trait Cell {
    fn place_ant(&mut self, ant: Box<&Ant>);
    fn pop_ant(&mut self) -> Option<Box<&Ant>>;
    fn ant_color(&self) -> Option<Color>;

    fn object(&self) -> Option<WorldObject>;
    fn occupied(&self) -> bool;

    fn mark_pheromone(&mut self, i: usize, color: Color);
    fn unmark_pheromone(&mut self, i: usize, color: Color);
    fn is_marked_index(&self, i: usize, color: Color) -> bool;
    fn is_marked_any(&self, color: Color) -> bool;
    fn pheromones(&self, color: Color) -> u8;

    fn has_food(&self) -> bool;
    fn pickup_food(&mut self) -> bool;
    fn drop_food(&mut self);
}

pub struct Map {
    entities: Vec<Ant>,
    cells: Vec<Box<dyn Cell>>,
    size: (usize, usize)
}
impl Map {
    // Créé une nouvelle map vide de taille donnée
    pub fn new_empty(size: (usize, usize)) -> Self {
        let mut s = Self {
            entities: Vec::new(),
            cells: Vec::with_capacity(size.0 * size.1),
            size
        };
        for c in s.cells.iter_mut() {
            *c = Box::new(EmptyCell::default())
        }
        s
    }

    // Charge une map depuis un fichier comme décrit dans
    // le sujet du projet
    pub fn load_file(path: &str) -> Self {
        let content = fs::read_to_string(path)
            .expect("Could not open map file");

        let mut map = Self {
            entities: Vec::new(),
            cells: Vec::with_capacity(content.len()),
            size: (0, 0)
        };
        let mut ants: Vec<RefCell<Ant>> = vec!();

        let mut x = 0;
        let mut y = 0;
        for c in content.chars() {
            x += 1;
            match c {
                '#' => map.cells.push(Box::new(ObstacleCell::default())),
                '.' => map.cells.push(Box::new(EmptyCell::default())),
                '+' => {
                    map.entities.push(
                        Ant::new(Color::Red, (x, y))
                    );
                    map.cells.push(
                        Box::new(
                            NestCell::new(Color::Red, Box::new(map.entities.last().unwrap()))
                        )
                    );
                },
                '-' => {
                    map.entities.push(
                        Ant::new(Color::Black, (x, y))
                    );
                    map.cells.push(
                        Box::new(
                            NestCell::new(Color::Black, Box::new(map.entities.last().unwrap()))
                        )
                    );
                },
                '\n' => {
                    y += 1;
                    x = 0;
                },
                _ => {
                    if c.is_digit(10) {
                        let food_units = c.to_digit(10).unwrap();
                        map.cells.push(Box::new(EmptyCell::with_food(food_units as usize)));
                    }
                }
            }
        }

        map.size = (x, y+1);
        map
    }

    // Renvoie un vecteur contenant les index des fourmis mortes
    // i.e entourées par au moins 5 fourmis adverses
    pub fn is_surrounded(&self, cell: (usize, usize)) -> bool {
        let (x, y) = cell;
        assert!(x < self.size.0 - 1 && y < self.size.1 - 1);
        let neighbors = [(x, y+1), (x, y-1), (x+1, y-1), (x-1, y+1), (x+1, y+1), (x-1, y-1)];

        if let Some(color) = self.cell(cell).ant_color() {
            let mut surrounding_count = 0;
            for neighbor in neighbors {
                if let Some(other_color) = self.cell(neighbor).ant_color() {
                    if other_color != color { surrounding_count += 1 }
                }
            }
            surrounding_count >= 5
        } else {
            false
        }
    }

    // Retire une fourmi de la cellule indiquée
    pub fn remove_ant(&mut self, cell: (usize, usize)) {
        self.cell_mut(cell).pop_ant();
    }

    // Récupère une référence à la cellule aux coordonnées données
    pub fn cell(&self, cell: (usize, usize)) -> &Box<dyn Cell> {
        let (x, y) = cell;
        let (size_x, size_y) = self.size;
        assert!(x < size_x && y < size_y);
        &self.cells[x * size_x + y]
    }
    pub fn cell_mut(&mut self, cell: (usize, usize)) -> &mut Box<dyn Cell> {
        let (x, y) = cell;
        let (size_x, size_y) = self.size;
        assert!(x < size_x && y < size_y);
        &mut self.cells[x * size_x + y]
    }

    // Vérifie une condition sur une cellule de la carte
    pub fn check_cond(&self, cell: (usize, usize), querier_color: Color, cond: Cond) -> bool {
        let c = self.cell(cell);
        let occupant = c.object();
        match (cond, occupant) {
            (Friend, Some(WorldObject::RedAnt)) => querier_color == Color::Red,
            (Friend, Some(WorldObject::BlackAnt)) => querier_color == Color::Black,
            (Foe, Some(WorldObject::RedAnt)) => querier_color == Color::Black,
            (Foe, Some(WorldObject::BlackAnt)) => querier_color == Color::Red,
            (FriendWithFood, _) => self.check_cond(cell, querier_color, Friend) && c.has_food(),
            (FoeWithFood, _) => self.check_cond(cell, querier_color, Foe) && c.has_food(),
            (Food, _) => c.has_food(),
            (Rock, Some(WorldObject::Rock)) => true,
            (Marker(i), _) => c.is_marked_index(i, querier_color),
            (FoeMarker, _) => c.is_marked_any(querier_color),
            (Home, Some(WorldObject::RedNest)) => querier_color == Color::Red,
            (Home, Some(WorldObject::BlackNest)) => querier_color == Color::Black,
            (FoeHome, Some(WorldObject::RedNest)) => querier_color == Color::Black,
            (FoeHome, Some(WorldObject::BlackNest)) => querier_color == Color::Red,
            _ => false
        }
    }

    pub fn mark_pheromone(&mut self, cell: (usize, usize), i: usize, color: Color) {
        let c = self.cell_mut(cell);
        c.mark_pheromone(i, color)
    }
    pub fn unmark_pheromone(&mut self, cell: (usize, usize), i: usize, color: Color) {
        let c = self.cell_mut(cell);
        c.unmark_pheromone(i, color);
    }

    pub fn pickup_food(&mut self, cell: (usize, usize)) -> bool {
        let c = self.cell_mut(cell);
        c.pickup_food()
    }
    pub fn drop_food(&mut self, cell: (usize, usize)) {
        let c = self.cell_mut(cell);
        c.drop_food()
    }

    pub fn try_move(&mut self, cell_from: (usize, usize), cell_to: (usize, usize)) -> bool {
        if !self.cell(cell_from).occupied() {
            if let Some(ant) = self.cell_mut(cell_from).pop_ant() {
                self.cell_mut(cell_to).place_ant(ant);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}
impl Debug for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, c) in self.cells.iter().enumerate() {
            if i % self.size.0 == 0 {
                write!(f, "\n")?
            }
            write!(f, "{} ", match c.object() {
                Some(WorldObject::RedNest) => '+',
                Some(WorldObject::BlackNest) => '-',
                Some(WorldObject::RedAnt) => '$',
                Some(WorldObject::BlackAnt) => '@',
                Some(WorldObject::Rock) => '#',
                None => '.'
            })?
        }
        write!(f, "")
    }
}

#[derive(Default, Debug)]
pub struct EmptyCell {
    food_units: usize,
    pheromones: u16,
    occupant: Option<Box<&Ant>>
}
impl EmptyCell {
    fn with_food(food_units: usize) -> Self {
        Self {
            food_units,
            .. Default::default()
        }
    }

    fn pheromone_offset(color: Color) -> usize {
        match color {
            Color::Red => 0,
            Color::Black => 8
        }
    }
}
impl Cell for EmptyCell {
    fn place_ant(&mut self, ant: Box<&Ant>) {
        self.occupant = Some(ant)
    }
    fn pop_ant(&mut self) -> Option<RefCell<Ant>> {
        let mut result = None;
        mem::swap(&mut result, &mut self.occupant);
        result
    }
    fn ant_color(&self) -> Option<Color> {
        if let Some(ant) = &self.occupant {
            Some(ant.color)
        } else {
            None
        }
    }

    fn object(&self) -> Option<WorldObject> {
        if let Some(occupant) = &self.occupant {
            match occupant.color {
                Color::Red => Some(WorldObject::RedAnt),
                Color::Black => Some(WorldObject::BlackAnt)
            }
        } else {
            None
        }
    }
    fn occupied(&self) -> bool {
        self.occupant.is_some()
    }

    fn mark_pheromone(&mut self, i: usize, color: Color) {
        assert!(i < 6);
        let offset = Self::pheromone_offset(color);
        self.pheromones | (1 << (i + offset));
        self.pheromones | (1 << (offset + 7));
    }

    fn unmark_pheromone(&mut self, i: usize, color: Color) {
        assert!(i < 6);
        let offset = Self::pheromone_offset(color);
        self.pheromones & !(1 << (i + offset));
        if self.pheromones(color) == 0 {
            self.pheromones & !(1 << (offset + 7));
        }
    }

    fn is_marked_index(&self, i: usize, color: Color) -> bool {
        assert!(i < 6);
        self.pheromones(color) & (1 << i) != 0
    }

    fn is_marked_any(&self, color: Color) -> bool {
        self.pheromones(color) & (1 << 7) != 0
    }

    fn pheromones(&self, color: Color) -> u8 {
        (match color {
            Color::Red => self.pheromones & 0b111111,
            Color::Black => (self.pheromones & 0b1111110000000) >> 7
        }) as u8
    }

    fn has_food(&self) -> bool {
        self.food_units != 0
    }

    fn pickup_food(&mut self) -> bool {
        if self.has_food() {
            self.food_units -= 1;
            true
        } else {
            false
        }
    }

    fn drop_food(&mut self) {
        // Le surplus est redistribué entre les ticks
        self.food_units += 1;
    }
}

#[derive(Default, Debug)]
struct ObstacleCell {}
impl Cell for ObstacleCell {
    fn place_ant(&mut self, _ant: RefCell<Ant>) {}
    fn pop_ant(&mut self) -> Option<RefCell<Ant>> {
        None
    }
    fn ant_color(&self) -> Option<Color> { None }

    fn object(&self) -> Option<WorldObject> {
        Some(WorldObject::Rock)
    }
    fn occupied(&self) -> bool {
        true
    }

    fn mark_pheromone(&mut self, _i: usize, _color: Color) {}
    fn unmark_pheromone(&mut self, _i: usize, _color: Color) {}
    fn is_marked_index(&self, _i: usize, _color: Color) -> bool { false }
    fn is_marked_any(&self, _color: Color) -> bool { false }
    fn pheromones(&self, _color: Color) -> u8 { 0 }

    fn has_food(&self) -> bool { false }
    fn pickup_food(&mut self) -> bool { false }
    fn drop_food(&mut self) {}
}

#[derive(Debug, Default)]
struct NestCell {
    color: Color,
    food_units: usize,
    occupant: Option<RefCell<Ant>>
}
impl NestCell {
    fn new(color: Color, ant: RefCell<Ant>) -> Self {
        Self {
            color,
            food_units: 0,
            occupant: Some(ant)
        }
    }
}
impl Cell for NestCell {
    fn place_ant(&mut self, ant: RefCell<Ant>) {
        self.occupant = Some(ant)
    }
    fn pop_ant(&mut self) -> Option<RefCell<Ant>> {
        let mut result = None;
        mem::swap(&mut result, &mut self.occupant);
        result
    }
    fn ant_color(&self) -> Option<Color> {
        if let Some(ant) = &self.occupant {
            Some(ant.color)
        } else {
            None
        }
    }

    fn object(&self) -> Option<WorldObject> {
        match self.color {
            Color::Red => Some(WorldObject::RedNest),
            Color::Black => Some(WorldObject::BlackNest)
        }
    }
    fn occupied(&self) -> bool {
        self.occupant.is_some()
    }

    fn mark_pheromone(&mut self, _i: usize, _color: Color) {}
    fn unmark_pheromone(&mut self, _i: usize, _color: Color) {}
    fn is_marked_index(&self, _i: usize, _color: Color) -> bool { false }
    fn is_marked_any(&self, _color: Color) -> bool { false }
    fn pheromones(&self, _color: Color) -> u8 { 0 }

    fn has_food(&self) -> bool {
        self.food_units != 0
    }
    fn pickup_food(&mut self) -> bool {
        if self.has_food() {
            self.food_units -= 1;
            true
        } else {
            false
        }
    }
    fn drop_food(&mut self) {
        self.food_units += 1;
    }
}
