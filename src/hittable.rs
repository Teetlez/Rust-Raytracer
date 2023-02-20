use std::{cmp::Ordering, sync::Arc};

use crate::{material::Material, ray::Ray};

use ultraviolet::{Vec3, Vec4};

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>>;

    fn bounding_box(&self) -> AABB;
}

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
}
impl Hittable for Sphere {
    #[inline]
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
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

    fn bounding_box(&self) -> AABB {
        AABB {
            minimum: self.center.truncated()
                - Vec3::new(self.center.w, self.center.w, self.center.w),
            maximum: self.center.truncated()
                + Vec3::new(self.center.w, self.center.w, self.center.w),
        }
    }
}

#[derive(Copy, Clone)]
pub struct AABB {
    minimum: Vec3,
    maximum: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> AABB {
        AABB {
            minimum: min,
            maximum: max,
        }
    }

    #[inline]
    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> bool {
        for a in 0..3 {
            let inv_d = 1.0 / ray.dir[a];
            let (t0, t1) = if inv_d < 0.0 {
                (
                    ((self.maximum[a] - ray.pos[a]) * inv_d),
                    ((self.minimum[a] - ray.pos[a]) * inv_d),
                )
            } else {
                (
                    ((self.minimum[a] - ray.pos[a]) * inv_d),
                    ((self.maximum[a] - ray.pos[a]) * inv_d),
                )
            };
            if f32::min(t1, t_max) <= f32::max(t0, t_min) {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn surrounding_box(box1: AABB, box2: AABB) -> AABB {
        let small = Vec3::new(
            f32::min(box1.minimum.x, box2.minimum.x),
            f32::min(box1.minimum.y, box2.minimum.y),
            f32::min(box1.minimum.z, box2.minimum.z),
        );

        let big = Vec3::new(
            f32::max(box1.maximum.x, box2.maximum.x),
            f32::max(box1.maximum.y, box2.maximum.y),
            f32::max(box1.maximum.z, box2.maximum.z),
        );

        AABB::new(small, big)
    }
}

#[derive(Clone)]
pub struct BVH {
    aabb_box: Arc<AABB>,
    children: (
        Arc<dyn Hittable + Send + Sync>,
        Arc<dyn Hittable + Send + Sync>,
    ),
}

impl BVH {
    pub fn new(objects: &mut [Arc<dyn Hittable + Send + Sync>]) -> BVH {
        let axis = fastrand::usize(0..3);

        objects.sort_by(|a, b| BVH::box_compare(a.clone(), b.clone(), axis));
        let n = objects.len();
        let (left, right): (
            Arc<dyn Hittable + Send + Sync>,
            Arc<dyn Hittable + Send + Sync>,
        ) = if objects.len() <= 2 {
            let left = Arc::clone(objects.first().unwrap());
            let right = Arc::clone(objects.last().unwrap());
            (left, right)
        } else {
            let mid = n / 2;
            let left = Arc::new(BVH::new(&mut objects[..mid]));
            let right = Arc::new(BVH::new(&mut objects[mid..]));
            (left, right)
        };
        BVH {
            aabb_box: Arc::new(AABB::surrounding_box(
                left.bounding_box(),
                right.bounding_box(),
            )),
            children: (left, right),
        }
    }

    #[inline]
    fn box_compare(
        a: Arc<dyn Hittable + Send + Sync>,
        b: Arc<dyn Hittable + Send + Sync>,
        axis: usize,
    ) -> Ordering {
        let box_a = a.bounding_box();
        let box_b = b.bounding_box();

        if box_a.minimum[axis] < box_b.maximum[axis] {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl Hittable for BVH {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
        if self.aabb_box.hit(ray, t_min, t_max) {
            let hit_left = self.children.0.hit(ray, t_min, t_max);
            let hit_right = self.children.1.hit(ray, t_min, t_max);
            match (hit_left, hit_right) {
                (Some(hit_left), Some(hit_right)) => {
                    if hit_left.t < hit_right.t {
                        Some(hit_left)
                    } else {
                        Some(hit_right)
                    }
                }
                (Some(hit), None) => Some(hit),
                (None, Some(hit)) => Some(hit),
                (_, _) => None,
            }
        } else {
            None
        }
    }

    #[inline]
    fn bounding_box(&self) -> AABB {
        *self.aabb_box
    }
}
