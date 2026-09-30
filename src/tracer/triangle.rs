use crate::material::Material;
use ultraviolet::Vec3;

use super::primitive::{GpuPrimitiveSource, ScenePrimitive};

#[derive(Debug, Copy, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normals: [Vec3; 3],
    pub material: Material,
    two_sided: bool,
}

impl Triangle {
    pub fn new(
        vertices: [Vec3; 3],
        normals: [Vec3; 3],
        two_sided: bool,
        material: Material,
    ) -> Self {
        Self {
            vertices,
            normals,
            material,
            two_sided,
        }
    }
}

impl GpuPrimitiveSource for Triangle {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        primitives.push(ScenePrimitive::Triangle {
            vertices: self.vertices,
            normals: self.normals,
            two_sided: self.two_sided,
            material: self.material,
        });
    }
}
