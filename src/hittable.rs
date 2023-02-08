use crate::{ray::Ray, vec3::Vec3};

#[derive(Copy, Clone)]
pub struct HitRecord {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

impl HitRecord {
    pub fn new(t: f32, point: Vec3, normal: Vec3) -> HitRecord {
        HitRecord { t, point, normal }
    }
}

#[derive(Copy, Clone)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Sphere {
        Sphere { center, radius }
    }

    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.pos - self.center;
        let a = ray.dir.length_sq();
        let half_b = oc.dot(ray.dir);
        let c = oc.length_sq() - (self.radius * self.radius);
        let disc = half_b * half_b - a * c;

        if disc > 0.0 {
            let mut temp = (-half_b - (half_b * half_b - a * c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord {
                    t: temp,
                    point: hit_point,
                    normal: ((1.0 / self.radius) * (hit_point - self.center)).normalize(),
                });
            }

            temp = (-half_b + (half_b * half_b - a * c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord {
                    t: temp,
                    point: hit_point,
                    normal: ((1.0 / self.radius) * (hit_point - self.center)).normalize(),
                });
            }
        }
        None
    }
}

pub struct World {
    hittables: Vec<Sphere>,
}

impl World {
    pub fn new(hittables: Vec<Sphere>) -> World {
        World { hittables }
    }

    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest = t_max;
        let mut possible_hit: Option<HitRecord> = None;
        for object in self.hittables.iter() {
            if let Some(hit) = object.hit(&ray, t_min, t_max) {
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
}
