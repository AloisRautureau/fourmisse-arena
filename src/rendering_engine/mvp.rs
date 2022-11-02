use nalgebra_glm::{identity, TMat4};

#[derive(Debug, Clone)]
pub struct ModelViewProjection {
    pub model: TMat4<f32>,
    pub view: TMat4<f32>,
    pub projection: TMat4<f32>
}
impl Default for ModelViewProjection {
    fn default() -> Self {
        Self {
            model: identity(),
            view: identity(),
            projection: identity()
        }
    }
}