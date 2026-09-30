use ultraviolet::Vec3;

use crate::material::Material;

#[derive(Clone, Copy)]
pub enum ScenePrimitive {
    Sphere {
        center: Vec3,
        radius: f32,
        material: Material,
    },
    Triangle {
        vertices: [Vec3; 3],
        normals: [Vec3; 3],
        two_sided: bool,
        material: Material,
    },
}

pub trait GpuPrimitiveSource {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>);
}