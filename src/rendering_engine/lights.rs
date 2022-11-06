use crate::rendering_engine::directional_fragment_shader;
use std::sync::Arc;
use vulkano::buffer::cpu_pool::CpuBufferPoolSubbuffer;
use vulkano::buffer::CpuBufferPool;
use vulkano::memory::pool::StandardMemoryPool;

#[derive(Default, Debug, Clone)]
pub struct AmbientLightSource {
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Default, Debug, Clone)]
pub struct DirectionalLightSource {
    pub color: [f32; 3],
    pub position: [f32; 3],
    pub intensity: f32,
}
impl DirectionalLightSource {
    pub fn generate_directional_buffer(
        &self,
        pool: &CpuBufferPool<directional_fragment_shader::ty::DirectionalLight>,
    ) -> Arc<
        CpuBufferPoolSubbuffer<
            directional_fragment_shader::ty::DirectionalLight,
            Arc<StandardMemoryPool>,
        >,
    > {
        let uniform_data = directional_fragment_shader::ty::DirectionalLight {
            color: self.color,
            intensity: self.intensity,
            position: self.position,
        };
        pool.from_data(uniform_data)
            .unwrap_or_else(|err| panic!("failed to create directional subbuffer: {:?}", err))
    }
}
