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
pub struct LightSource {
    pub color: [f32; 3],
    pub vector: [f32; 4],
}
impl LightSource {
    pub fn generate_directional_buffer(
        &self,
        pool: &CpuBufferPool<directional_fragment_shader::ty::LightSource>,
    ) -> Arc<
        CpuBufferPoolSubbuffer<
            directional_fragment_shader::ty::LightSource,
            Arc<StandardMemoryPool>,
        >,
    > {
        let uniform_data = directional_fragment_shader::ty::LightSource {
            color: self.color,
            vector: self.vector,
        };
        pool.from_data(uniform_data)
            .unwrap_or_else(|err| panic!("failed to create directional subbuffer: {:?}", err))
    }
}
