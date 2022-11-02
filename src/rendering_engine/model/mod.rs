use nalgebra_glm::{identity, rotate_normalized_axis, TMat4, translate, TVec3};
use crate::rendering_engine::model::loader::ObjLoader;
use crate::Vertex;

mod loader;
pub mod vertex;

pub struct Model {
    data: Vec<Vertex>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    model: TMat4<f32>,
    requires_update: bool
}
impl Model {
    pub fn load(path: &str, color: Option<[f32; 3]>, invert_winding: bool) -> Self {
        let loader = ObjLoader::load(path, color, invert_winding);
        Self {
            data: loader.get_vertices(),
            .. Default::default()
        }
    }

    pub fn data(&self) -> Vec<Vertex> {
        self.data.clone()
    }
    pub fn model_matrix(&mut self) -> TMat4<f32> {
        if self.requires_update {
            self.model = self.translation * self.rotation;
            self.requires_update = false
        }
        self.model
    }
    pub fn rotate(&mut self, radians: f32, axis: &TVec3<f32>) {
        self.rotation = rotate_normalized_axis(&self.rotation, radians, axis);
        self.requires_update = true
    }
    pub fn translate(&mut self, translation: &TVec3<f32>) {
        self.translation = translate(&self.translation, translation);
        self.requires_update = true
    }
}
impl Default for Model {
    fn default() -> Self {
        Self {
            data: vec!(),
            translation: identity(),
            rotation: identity(),
            model: identity(),
            requires_update: false
        }
    }
}