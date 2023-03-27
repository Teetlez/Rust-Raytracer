use crate::{material::Material, ray::Ray};

use ultraviolet::Vec3;

use super::cube::Aabb;

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;

    fn bounding_box(&self) -> Aabb;
}

#[derive(Clone)]
pub struct HitRecord<'a> {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: &'a Material,
}

impl HitRecord<'_> {
    pub fn new(t: f32, point: Vec3, normal: Vec3, material: &Material) -> HitRecord {
        HitRecord {
            t,
            point,
            normal,
            material,
        }
    }
}
