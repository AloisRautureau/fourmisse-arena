use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{make_vec3, TVec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position, normal, colour);
impl Vertex {
    pub fn translate(&mut self, translation: &TVec3<f32>) {
        let new_pos = make_vec3(&self.position) + translation;
        self.position = new_pos.data.0[0];
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct DummyVertex {
    pub position: [f32; 2],
}
impl DummyVertex {
    // Useful to avoid having to pass geometry to every
    // render pass, by instead applying fragment
    // shaders to every pixel on the screen
    pub fn cover_viewport() -> [Self; 6] {
        [
            Self {
                position: [-1.0, -1.0],
            },
            Self {
                position: [-1.0, 1.0],
            },
            Self {
                position: [1.0, 1.0],
            },
            Self {
                position: [-1.0, -1.0],
            },
            Self {
                position: [1.0, 1.0],
            },
            Self {
                position: [1.0, -1.0],
            },
        ]
    }
}
vulkano::impl_vertex!(DummyVertex, position);
