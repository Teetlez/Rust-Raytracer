use crate::{material::Material, ray::Ray};

use ultraviolet::{Vec3, Vec4};

use super::{
    cube::Aabb,
    hittable::{HitRecord, Hittable},
};

#[derive(Copy, Clone)]
pub struct Sphere {
    pub center: Vec4,
    pub material: Material,
}

impl Sphere {
    pub fn new(center: (f32, f32, f32), radius: f32, material: Material) -> Sphere {
        Sphere {
            center: Vec4::new(center.0, center.1, center.2, radius),
            material,
        }
    }
}
impl Hittable for Sphere {
    #[inline]
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.pos - self.center.truncated();
        let half_b = oc.dot(ray.dir);
        let c = oc.mag_sq() - (self.center.w * self.center.w);
        let disc = half_b * half_b - c;

        if disc > 0.0 {
            let h = disc.sqrt();
            let mut temp = -half_b - h;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    ((1.0 / self.center.w) * (hit_point - self.center.truncated())).normalized(),
                    self.material,
                ));
            }

            temp = -half_b + h;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    ((1.0 / self.center.w) * (hit_point - self.center.truncated())).normalized(),
                    self.material,
                ));
            }
        }
        None
    }

    fn bounding_box(&self) -> Aabb {
        Aabb {
            min: self.center.truncated()
                - Vec3::new(self.center.w, self.center.w, self.center.w).abs(),
            max: self.center.truncated()
                + Vec3::new(self.center.w, self.center.w, self.center.w).abs(),
        }
    }
}
