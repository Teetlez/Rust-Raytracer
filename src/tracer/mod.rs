pub mod bvh;
pub mod cube;
pub mod hittable;
pub mod mesh;
pub mod sphere;
pub mod triangle;

use crate::{gpu_scene::GpuScene, tracer::hittable::Hittable};

impl bvh::Bvh {
    pub fn to_gpu_scene(&self) -> GpuScene {
        let mut primitives = Vec::new();
        self.append_gpu_primitives(&mut primitives);
        GpuScene::from_cpu_primitives(primitives)
    }
}
