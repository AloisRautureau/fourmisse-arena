use crate::rendering_engine::model::loader::ObjLoader;
use crate::Vertex;

mod loader;
pub mod vertex;

pub struct Model(Vec<Vertex>);
impl Model {
    pub fn load(path: &str, colour: Option<[f32; 3]>, invert_winding: bool) -> Self {
        let loader = ObjLoader::load(path, colour, invert_winding);
        Self(loader.get_vertices())
    }

    pub fn data(&self) -> &Vec<Vertex> {
        &self.0
    }

    pub fn set_colour(&mut self, colour: [f32; 3]) {
        for v in &mut self.0 {
            v.colour = colour;
        }
    }
}
