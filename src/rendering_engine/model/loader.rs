use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::Vertex;


pub struct ObjLoader {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    text: Vec<[f32; 3]>,
    faces: Vec<RawFace>,

    color: [f32; 3],
    invert_winding: bool
}
impl ObjLoader {
    pub fn load(path: &str, color: Option<[f32; 3]>, invert_winding: bool) -> ObjLoader {
        let f = File::open(path)
            .unwrap_or_else(|err| panic!("failed to open file {}: {:?}", path, err));
        let bufreader = BufReader::new(f);
        let mut vertices = vec!();
        let mut normals = vec!();
        let mut text = vec!();
        let mut faces = vec!();

        // Parse the file
        for line in bufreader.lines() {
            if let Ok(line) = line {
                if line.len() > 2 {
                    match line.split_at(2) {
                        ("v ", x) => vertices.push(Self::parse_raw_vertex(x)),
                        ("vn", x) => normals.push(Self::parse_raw_vertex(x)),
                        ("vt", x) => text.push(Self::parse_raw_vertex(x)),
                        ("f ", x) => faces.push(RawFace::new(x, invert_winding)),
                        _ => ()
                    }
                }
            }
        }

        Self {
            vertices,
            normals,
            text,
            faces,

            color: color.unwrap_or([1f32; 3]),
            invert_winding
        }
    }

    pub fn get_vertices(&self) -> Vec<Vertex> {
        self.faces.iter()
            .flat_map(|f| {
                let vertex_indices = f.vertex_indices;
                let normal_indices = f.normal_indices.unwrap();

                println!("{:?}", normal_indices);

                [
                    Vertex {
                        position: *self.vertices.get(vertex_indices[0]).unwrap(),
                        normal: *self.normals.get(normal_indices[0]).unwrap(),
                        color: self.color
                    },
                    Vertex {
                        position: *self.vertices.get(vertex_indices[1]).unwrap(),
                        normal: *self.normals.get(normal_indices[1]).unwrap(),
                        color: self.color
                    },
                    Vertex {
                        position: *self.vertices.get(vertex_indices[2]).unwrap(),
                        normal: *self.normals.get(normal_indices[2]).unwrap(),
                        color: self.color
                    },
                ]
            })
            .collect()
    }

    fn parse_raw_vertex(input: &str) -> [f32; 3] {
        let values: Vec<f32> = input.split_whitespace()
            .map(|v| v.parse::<f32>().unwrap_or_else(|_| panic!("invalid values for a vertex: {}", input)))
            .collect();
        [
            *values.get(0).unwrap_or(&0.0),
            *values.get(1).unwrap_or(&0.0),
            *values.get(0).unwrap_or(&0.0),
        ]
    }
}

// Represents a raw triangle as indices to its vertices/normals/text
struct RawFace {
    pub vertex_indices: [usize; 3],
    pub normal_indices: Option<[usize; 3]>,
    pub text_indices: Option<[usize; 3]>
}
impl RawFace {
    pub fn new(input: &str, invert_winding: bool) -> Self {
        let arguments: Vec<&str> = input.split_whitespace().collect();
        Self {
            vertex_indices: RawFace::parse(arguments.clone(), 0, invert_winding).unwrap(),
            normal_indices: RawFace::parse(arguments.clone(), 2, invert_winding),
            text_indices: RawFace::parse(arguments.clone(), 1, invert_winding),
        }
    }

    fn parse(inputs: Vec<&str>, index: usize, invert_winding: bool) -> Option<[usize; 3]> {
        let f1: Vec<&str> = inputs.get(0).unwrap().split("/").collect();
        let f2: Vec<&str> = inputs.get(1).unwrap().split("/").collect();
        let f3: Vec<&str> = inputs.get(2).unwrap().split("/").collect();
        let a1 = f1.get(index).unwrap().clone();
        let a2 = f2.get(index).unwrap().clone();
        let a3 = f3.get(index).unwrap().clone();
        match a1 {
            "" => None,
            _ => {
                let p1: usize = a1.parse().unwrap();
                let (p2, p3): (usize, usize) = if invert_winding {
                    (a3.parse().unwrap(), a2.parse().unwrap())
                } else {
                    (a2.parse().unwrap(), a3.parse().unwrap())
                };
                Some([p1 - 1, p2 - 1, p3 - 1]) // .obj files aren't 0-index
            }
        }
    }
}