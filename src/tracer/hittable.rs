use crate::{material::Material, ray::Ray};

use ultraviolet::Vec3;

use super::cube::Aabb;

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

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>>;

    fn bounding_box(&self) -> Aabb;

    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>);
}

#[derive(Clone, Copy)]
pub struct HitRecord<'a> {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: &'a Material,
}

impl HitRecord<'_> {
    pub fn new(t: f32, point: Vec3, normal: Vec3, material: &Material) -> HitRecord<'_> {
        HitRecord {
            t,
            point,
            normal,
            material,
        }
    }
}
