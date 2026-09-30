use crate::{material::Material, ray::Ray};

use ultraviolet::Vec3;

use super::{
    cube::Aabb,
    hittable::{HitRecord, Hittable, ScenePrimitive},
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
impl Hittable for Sphere {
    #[inline]
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
        let oc = ray.pos - self.center;
        let half_b = oc.dot(ray.dir);
        let disc = half_b.powi(2) - (oc.mag_sq() - (self.radius.powi(2)));

        if disc > 0.0 {
            let h = disc.sqrt();
            let mut temp = -half_b - h;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    (hit_point - self.center).normalized(),
                    &self.material,
                ));
            }

            temp = -half_b + h;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    (hit_point - self.center).normalized(),
                    &self.material,
                ));
            }
        }
        None
    }

    fn bounding_box(&self) -> Aabb {
        Aabb {
            min: self.center - Vec3::one() * self.radius.abs(),
            max: self.center + Vec3::one() * self.radius.abs(),
        }
    }

    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        primitives.push(ScenePrimitive::Sphere {
            center: self.center,
            radius: self.radius,
            material: self.material,
        });
    }
}
