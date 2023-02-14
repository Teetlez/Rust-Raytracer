use crate::{material::Material, ray::Ray};

use ultraviolet::{Vec3, Vec4};

#[derive(Copy, Clone)]
pub struct HitRecord<'obj> {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: &'obj Material,
}

impl HitRecord<'_> {
    pub fn new(t: f32, point: Vec3, normal: Vec3, material: &'_ Material) -> HitRecord<'_> {
        HitRecord {
            t,
            point,
            normal,
            material,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Sphere {
    pub center: Vec4,
    pub material: Material,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, material: Material) -> Sphere {
        Sphere {
            center: Vec4::new(center.x, center.y, center.z, radius),
            material,
        }
    }

    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
        let oc = ray.pos - self.center.truncated();
        let half_b = oc.dot(ray.dir);
        let c = oc.mag_sq() - (self.center.w * self.center.w);
        let disc = half_b * half_b - c;

        if disc > 0.0 {
            let mut temp = -half_b - (half_b * half_b - c).sqrt();
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    ((1.0 / self.center.w) * (hit_point - self.center.truncated())).normalized(),
                    &self.material,
                ));
            }

            temp = -half_b + (half_b * half_b - c).sqrt();
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    ((1.0 / self.center.w) * (hit_point - self.center.truncated())).normalized(),
                    &self.material,
                ));
            }
        }
        None
    }
}

pub struct World {
    pub hittables: Vec<Sphere>,
}

impl World {
    pub fn new(hittables: Vec<Sphere>) -> World {
        World { hittables }
    }

    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
        let mut closest = t_max;
        let mut possible_hit: Option<HitRecord> = None;
        for object in self.hittables.iter() {
            if let Some(hit) = object.hit(ray, t_min, t_max) {
                closest = if hit.t < closest {
                    possible_hit = Some(hit);
                    hit.t
                } else {
                    closest
                }
            }
        }
        possible_hit
    }

    pub fn add(&mut self, sphere: Sphere) {
        self.hittables.push(sphere);
    }
}
