use nalgebra_glm::{identity, look_at, vec3, TMat4, TVec3};
use vulkano::buffer::CpuAccessibleBuffer;

#[derive(Debug, Clone)]
pub struct ViewProjection {
    pub camera_position: TVec3<f32>,

    pub view: TMat4<f32>,
    pub projection: TMat4<f32>,
}
impl ViewProjection {
    pub fn move_camera(&mut self, delta: &TVec3<f32>) {
        self.camera_position += delta;

        // Let's us create a normalized vector
        let dist = (1_f32 / 3_f32).sqrt();
        self.view = look_at(
            &self.camera_position,
            &(self.camera_position - vec3(dist, dist, dist)),
            &vec3(0.0, 1.0, 0.0),
        );
    }
}
impl Default for ViewProjection {
    fn default() -> Self {
        let camera_position = vec3(30.0, 30.0, 30.0);
        let dist = (1_f32 / 3_f32).sqrt();
        Self {
            camera_position,
            view: look_at(
                &camera_position,
                &(camera_position - vec3(dist, dist, dist)),
                &vec3(0.0, 1.0, 0.0),
            ),
            projection: identity(),
        }
    }
}
