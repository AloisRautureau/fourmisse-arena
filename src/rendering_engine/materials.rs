#[derive(Default, Debug, Clone)]
pub struct Material {
    pub colour: [f32; 3],
    pub shininess: f32,
    pub specular_intensity: f32,
}