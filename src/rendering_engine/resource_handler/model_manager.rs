use crate::rendering_engine::resource_handler::{ResourceHandle, ResourceVec};
use crate::rendering_engine::{Model, Vertex};

#[derive(Default)]
pub struct ModelVec {
    models: Vec<Model>,
}
impl ResourceVec for ModelVec {
    // Loads a model and returns the corresponding resource handle
    fn load(&mut self, path: &str) -> ResourceHandle {
        let index = self.models.len();
        self.models.push(Model::load(path, true));
        ResourceHandle(index)
    }
}
impl ModelVec {
    pub fn fetch_model_vertices(&self, handle: &ResourceHandle) -> Vec<Vertex> {
        self.models[handle.index()].data().clone()
    }
}
