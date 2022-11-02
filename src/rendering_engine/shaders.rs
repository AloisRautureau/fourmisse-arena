pub mod deferred_vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/deferred.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}
pub mod deferred_fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/deferred.frag",
    }
}

pub mod ambient_vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/ambient.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}
pub mod ambient_fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/ambient.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}

pub mod directional_vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/directional.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}
pub mod directional_fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/directional.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}