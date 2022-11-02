use std::sync::Arc;
use vulkano::buffer::cpu_pool::CpuBufferPoolSubbuffer;
use vulkano::buffer::CpuBufferPool;
use vulkano::memory::pool::StandardMemoryPool;
use crate::{ambient_fragment_shader, directional_fragment_shader};

#[derive(Default, Debug, Clone)]
pub struct AmbientLightSource {
    pub color: [f32; 3],
    pub intensity: f32
}
impl AmbientLightSource {
    pub fn generate_ambient_buffer(
        &self,
        pool: &CpuBufferPool<ambient_fragment_shader::ty::AmbientLight>
    ) -> Arc<CpuBufferPoolSubbuffer<
        ambient_fragment_shader::ty::AmbientLight,
        Arc<StandardMemoryPool>
    >> {
        let uniform_data = ambient_fragment_shader::ty::AmbientLight {
            color: self.color.into(),
            intensity: self.intensity.into(),
        };
        pool.from_data(uniform_data)
            .unwrap_or_else(|err| panic!("failed to create ambient subbuffer: {:?}", err))
    }
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
        pool: &CpuBufferPool<directional_fragment_shader::ty::DirectionalLight>
    ) -> Arc<CpuBufferPoolSubbuffer<
        directional_fragment_shader::ty::DirectionalLight,
        Arc<StandardMemoryPool>
    >> {
        let uniform_data = directional_fragment_shader::ty::DirectionalLight {
            color: self.color.into(),
            intensity: self.intensity.into(),
            position: self.position.into(),
        };
        pool.from_data(uniform_data)
            .unwrap_or_else(|err| panic!("failed to create directional subbuffer: {:?}", err))
    }
}