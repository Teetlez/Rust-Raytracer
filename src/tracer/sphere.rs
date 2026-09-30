use crate::material::Material;

use ultraviolet::Vec3;

use super::{
    primitive::{GpuPrimitiveSource, ScenePrimitive},
};

#[derive(Copy, Clone)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl Sphere {
    pub fn new(center: (f32, f32, f32), radius: f32, material: Material) -> Sphere {
        Sphere {
            center: Vec3::new(center.0, center.1, center.2),
            radius,
            material,
        }
    }
}
impl GpuPrimitiveSource for Sphere {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        primitives.push(ScenePrimitive::Sphere {
            center: self.center,
            radius: self.radius,
            material: self.material,
        });
    }
}
