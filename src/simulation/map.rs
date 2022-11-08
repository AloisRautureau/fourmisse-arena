use super::ant::{Ant, Colour};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::cmp::{min};
use std::fmt::{Debug, Formatter};

use crate::rendering_engine::{
    Material, Model, RenderingEngine, ResourceHandle, ResourceHandler, Vertex,
};
use crate::simulation::instruction::Cond;
use crate::simulation::{Simulation, HEXAGON_HEIGHT, HEXAGON_RADIUS, HEXAGON_WIDTH};
use nalgebra_glm::{identity, inverse_transpose, make_vec4, translate, vec3, vec3_to_vec4, vec4_to_vec3, TMat4, pi};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub type AntRef = Arc<Mutex<Ant>>;

#[derive(Clone)]
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
impl Cell {
    // Returns the correct material to use when rendering
    pub fn material(&self) -> Material {
        match self {
            Self::Empty { .. } => Material {
                colour: [0.5961, 0.5922, 0.1020],
                shininess: 32.0,
                specular_intensity: 1.0
            },
            Self::Nest { colour, .. } => Material {
                colour: colour.rgb(),
                shininess: 32.0,
                specular_intensity: 1.0
            },
            Self::Obstacle => Material {
                colour: [0.4098, 0.1627, 0.0],
                shininess: 32.0,
                specular_intensity: 1.0
            },
        }
    }
}

// A map contains a matrix of cells, which can be obstacles or empty.
// Empty cells can have at most 9 units of food on them
pub struct Map {
    cells: Vec<Cell>,
    kill_marks: Vec<bool>,
    transform_matrices: Vec<(TMat4<f32>, TMat4<f32>)>,
    pub size: (usize, usize),
}
impl Map {
    // Loads a map from a file
    // Returns loaded map, as well as a vector of ants derived from it
    pub fn load_file(path: &str) -> (Self, Vec<AntRef>) {
        let mut ants = vec![];
        let mut map = Self {
            cells: Vec::new(),
            kill_marks: Vec::new(),
            transform_matrices: Vec::new(),
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
        let (mut x, mut y) = (0, 0);
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
                let added_cell = match c {
                    '#' => {
                        map.cells.push(Cell::Obstacle);
                        true
                    }
                    '.' => {
                        map.cells.push(Cell::Empty {
                            food: 0,
                            occupant: None,
                            markers: [0; 2],
                        });
                        true
                    }
                    '+' => {
                        let new_ant = Ant::new(get_id(), Colour::Red, (x, y));
                        let ant_ref = Arc::new(Mutex::new(new_ant));
                        ants.push(Arc::clone(&ant_ref));
                        map.cells.push(Cell::Nest {
                            colour: Colour::Red,
                            food: 0,
                            occupant: Some(Arc::clone(&ant_ref)),
                            markers: [0; 2],
                        });
                        true
                    }
                    '-' => {
                        let new_ant = Ant::new(get_id(), Colour::Black, (x, y));
                        let ant_ref = Arc::new(Mutex::new(new_ant));
                        ants.push(Arc::clone(&ant_ref));
                        map.cells.push(Cell::Nest {
                            colour: Colour::Black,
                            food: 0,
                            occupant: Some(Arc::clone(&ant_ref)),
                            markers: [0; 2],
                        });
                        true
                    }
                    ' ' => false,
                    '\n' => {
                        y += 1;
                        x = 0;
                        false
                    }
                    _ => {
                        if c.is_ascii_digit() {
                            let food = c.to_digit(10).unwrap() as u8;
                            map.cells.push(Cell::Empty {
                                food,
                                occupant: None,
                                markers: [0; 2],
                            });
                            true
                        } else {
                            false
                        }
                    }
                };

                if added_cell {
                    let mut render_position = Simulation::render_position((x, y));
                    render_position.y = if matches!(map.cells.last().unwrap(), Cell::Obstacle) {
                        0.2
                    } else {
                        -HEXAGON_RADIUS / 2_f32 - 0.2
                    };
                    let model_matrix = translate(&identity(), &render_position);
                    let normals_matrix = inverse_transpose(model_matrix);
                    map.transform_matrices.push((model_matrix, normals_matrix));

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

    pub fn move_to(&mut self, moving_colour: Colour, from: (usize, usize), to: (usize, usize)) -> bool {
        if self.occupied(to) {
            false
        } else {
            // Take the ant from the source cell
            let ant = if let Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } = &mut self[from] {
                occupant.take()
            } else {
                None
            };

            // Then move it to its destination
            if let Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } = &mut self[to] {
                *occupant = ant
            }

            // We should then check if this move kills any surrounding ants, or the moved ant itself
            self.mark_killed_ants(moving_colour, to);

            true
        }
    }
    // Checks and removes killed ants, starting the checkup from a given starting point
    // This lets us check after every move, setting the destination of the move as the start point
    fn mark_killed_ants(&mut self, colour: Colour, position: (usize, usize)) {
        let mut surrounding_enemies = 0;
        let mut to_check_after = vec!();

        if let Cell::Empty { occupant: Some(ant_ref), .. } | Cell::Nest { occupant: Some(ant_ref), .. } = &self[position] {
            let surrounding_cells = self.surroundings(position);

            let mut last_ally = -1;
            let mut ennemies_since_last_ally = 0;
            for (i, (cell, _)) in surrounding_cells.iter().enumerate() {
                if let Cell::Empty { occupant: Some(other_ant_ref), .. } | Cell::Nest { occupant: Some(other_ant_ref), .. } = cell {
                    let other_ant_colour = if let Ok(a) = other_ant_ref.try_lock() { a.colour } else { colour.opposite() };
                    if other_ant_colour == colour {
                        if (last_ally == 2 && ennemies_since_last_ally == 1) || (last_ally == 3 && ennemies_since_last_ally == 2) {
                            // In this configuration, an enemy ant might be captured, so we must check
                            // using them as starting points
                            for j in i - ennemies_since_last_ally .. i {
                                to_check_after.push(surrounding_cells[j].1)
                            }
                        }
                        last_ally = 0;
                        ennemies_since_last_ally = 0;
                    } else {
                        surrounding_enemies += 1;
                        ennemies_since_last_ally += 1;
                        last_ally += if last_ally != -1 { 1 } else { 0 }
                    }
                }
            }
        }

        if surrounding_enemies >= 5 {
            // Marks the cell for cleanup
            let index = self.position_to_index(position);
            self.kill_marks[index] = true
        } else {
            for pos in to_check_after {
                self.mark_killed_ants(colour.opposite(), pos)
            }
        }
    }
    pub fn cleanup_killed_ants(&mut self) {
        let marked_cells = self.cells.iter_mut().zip(&self.kill_marks)
            .filter_map(|(c, m)| if *m { Some(c) } else { None });
        for cell in marked_cells {
            if let Cell::Empty { occupant, .. } | Cell::Nest { occupant, .. } = cell {
                *occupant = None
            }
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
        assert!(cell.0 < self.size.0 && cell.1 < self.size.1);
        match condition {
            Cond::Friend => if let Cell::Empty { occupant: Some(ant), .. } | Cell::Nest { occupant: Some(ant), .. } = &self[cell] {
                ant.lock().unwrap().colour == perspective

            } else {
                false
            },
            Cond::Foe => if let Cell::Empty { occupant: Some(ant), .. } | Cell::Nest { occupant: Some(ant), .. } = &self[cell] {
                ant.lock().unwrap().colour != perspective
            } else {
                false
            },
            Cond::FriendWithFood => if let Cell::Empty { occupant: Some(ant), .. } | Cell::Nest { occupant: Some(ant), .. } = &self[cell] {
                let ant = ant.lock().unwrap();
                ant.colour == perspective && ant.has_food
            } else {
                false
            },
            Cond::FoeWithFood => if let Cell::Empty { occupant: Some(ant), .. } | Cell::Nest { occupant: Some(ant), .. } = &self[cell] {
                let ant = ant.lock().unwrap();
                ant.colour != perspective && ant.has_food
            } else {
                false
            },
            Cond::Food => if let Cell::Empty { food, .. } | Cell::Nest { food, .. } = self[cell] {
                food != 0
            } else {
                false
            },
            Cond::Rock => matches!(self[cell], Cell::Obstacle),
            Cond::Marker(i) => if let Cell::Empty { markers, .. } | Cell::Nest { markers, .. } = self[cell] {
                markers[perspective.as_index()] & (1 << i) != 0
            } else {
                false
            },
            Cond::FoeMarker => if let Cell::Empty { markers, .. } | Cell::Nest { markers, .. } = self[cell] {
                markers[perspective.opposite().as_index()] != 0
            } else {
                false
            },
            Cond::Home => if let Cell::Nest { colour, .. } = self[cell] {
                colour == perspective
            } else {
                false
            },
            Cond::FoeHome => if let Cell::Nest { colour, .. } = self[cell] {
                colour != perspective
            } else {
                false
            },
        }
    }

    // Counts the total food count in both sides' nests
    pub fn score(&self) -> (u32, u32) {
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

    // Given a position, returns the state of the six surrounding cells clockwise starting from the
    // easter position, as well as their position
    pub fn surroundings(&self, position: (usize, usize)) -> [(&Cell, (usize, usize)); 6] {
        assert!(position.0 < self.size.0 - 1 && position.0 > 0 && position.1 > 0 && position.1 < self.size.1 - 1);
        let (x, y) = position;

        [
            (&self[(x + 1, y)], (x + 1, y)),
            (&self[(x + 1, y - 1)], (x + 1, y -1)),
            (&self[(x - 1, y - 1)], (x - 1, y - 1)),
            (&self[(x - 1, y)], (x - 1, y)),
            (&self[(x - 1, y + 1)], (x - 1, y + 1)),
            (&self[(x + 1, y + 1)], (x + 1, y + 1))
        ]
    }

    // Returns a vector of AntRef containing the ants currently on the map
    pub fn ants(&self) -> Vec<AntRef> {
        let mut ants = vec!();
        for cell in &self.cells {
            if let Cell::Empty { occupant: Some(ant), .. } | Cell::Nest { occupant: Some(ant), .. } = cell {
                ants.push(ant.clone())
            }
        }
        ants
    }

    fn position_to_index(&self, position: (usize, usize)) -> usize {
        let (x, y) = position;
        let (size_x, size_y) = self.size;
        assert!(x < size_x && y < size_y);
        y * size_x + x
    }

    // Returns a set of vertices representing the entire map
    // as one model
    pub fn render(
        &self,
        renderer: &mut RenderingEngine,
        tile_model_handle: ResourceHandle,
        food_model_handle: ResourceHandle,
        resource_handler: &ResourceHandler,
    ) {
        for (cell, (model, normal)) in self.cells.iter().zip(&self.transform_matrices) {
            renderer.add_model(
                tile_model_handle,
                resource_handler,
                (*model, *normal),
                &cell.material(),
            );

            if let Cell::Empty { food, .. } | Cell::Nest { food, .. } = cell {
                if *food > 0 {
                    let base_y = model.m24 + 0.075 + HEXAGON_RADIUS / 2f32;
                    Simulation::render_food_piece(renderer, food_model_handle, resource_handler, vec3(model.m14, base_y, model.m34), 0f32);
                    let rotation = pi::<f32>() / 3f32;
                    for i in 0..min(*food - 1, 5) {
                        Simulation::render_food_piece(
                            renderer,
                            food_model_handle,
                            resource_handler,
                            vec3(
                                model.m14 + (rotation * i as f32).cos() * HEXAGON_WIDTH / 4f32,
                                base_y,
                                model.m34 + (rotation * i as f32).sin() * HEXAGON_HEIGHT / 4f32
                            ),
                            0f32
                        )
                    }
                }
            }
        }
    }
    pub fn render_light(&self, renderer: &mut RenderingEngine) {
        for (cell, (model, _)) in self.cells.iter().zip(&self.transform_matrices) {
            if let Cell::Empty { food, .. } | Cell::Nest { food, .. } = cell {
                if *food > 0 {
                    let base_y = model.m24 + 0.075 + HEXAGON_RADIUS / 2f32;
                    Simulation::render_food_light(renderer, vec3(model.m14, base_y, model.m34));
                    let rotation = pi::<f32>() / 3f32;
                    for i in 0..min(*food - 1, 5) {
                        Simulation::render_food_light(
                            renderer,
                            vec3(
                                model.m14 + (rotation * i as f32).cos() * HEXAGON_WIDTH / 4f32,
                                base_y,
                                model.m34 + (rotation * i as f32).sin() * HEXAGON_HEIGHT / 4f32
                            )
                        )
                    }
                }
            }
        }
    }
}
impl Index<(usize, usize)> for Map {
    type Output = Cell;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.cells[self.position_to_index(index)]
    }
}

impl IndexMut<(usize, usize)> for Map {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let i = self.position_to_index(index);
        &mut self.cells[i]
    }
}
impl Debug for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, c) in self.cells.iter().enumerate() {
            if i % self.size.0 == 0 {
                writeln!(f)?;
                if (i / self.size.0) % 2 == 0 {
                    write!(f, " ")?
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
                        if ant.lock().unwrap().colour == Colour::Black {
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
                        if ant.lock().unwrap().colour == Colour::Black {
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
