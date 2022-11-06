use crate::rendering_engine::model::loader::ObjLoader;
use crate::Vertex;

mod loader;
pub mod vertex;

pub struct Model(Vec<Vertex>);
impl Model {
    pub fn load(path: &str, invert_winding: bool) -> Self {
        let loader = ObjLoader::load(path, invert_winding);
        Self(loader.get_vertices())
    }

    pub fn data(&self) -> Vec<Vertex> {
        self.0.clone()
    }
}
