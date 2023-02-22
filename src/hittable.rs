use std::{cmp::Ordering, sync::Arc};

use crate::{material::Material, ray::Ray};

use ultraviolet::{Vec3, Vec4};

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>>;

    fn bounding_box(&self) -> Aabb;
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
    pub fn new(center: (f32, f32, f32), radius: f32, material: Material) -> Sphere {
        Sphere {
            center: Vec4::new(center.0, center.1, center.2, radius),
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
            let h = disc.sqrt();
            let mut temp = -half_b - h;
            if temp < t_max && temp > t_min {
                let hit_point = ray.at(temp);
                return Some(HitRecord::new(
                    temp,
                    hit_point,
                    ((1.0 / self.center.w) * (hit_point - self.center.truncated())).normalized(),
                    &self.material,
                ));
            }

            temp = -half_b + h;
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

    fn bounding_box(&self) -> Aabb {
        Aabb {
            minimum: self.center.truncated()
                - Vec3::new(self.center.w, self.center.w, self.center.w),
            maximum: self.center.truncated()
                + Vec3::new(self.center.w, self.center.w, self.center.w),
        }
    }
}

pub struct ABox {
    pub minimum: Vec3,
    pub maximum: Vec3,
    pub material: Material,
}

impl ABox {
    pub fn new(center: (f32, f32, f32), size: (f32, f32, f32), mat: Material) -> ABox {
        ABox {
            minimum: Vec3::new(
                center.0 - (size.0 * 0.5),
                center.1 - (size.1 * 0.5),
                center.2 - (size.2 * 0.5),
            ),
            maximum: Vec3::new(
                center.0 + (size.0 * 0.5),
                center.1 + (size.1 * 0.5),
                center.2 + (size.2 * 0.5),
            ),
            material: mat,
        }
    }
}

impl Hittable for ABox {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        for a in 0..3 {
            if ray.dir[a].abs() < f32::EPSILON {
                if ray.pos[a] < self.minimum[a] || ray.pos[a] > self.maximum[a] {
                    return None;
                }
            } else {
                let inv_d = ray.dir[a].recip();
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
                t_near = t_near.max(t0);
                t_far = t_far.min(t1);
                if t_near > t_far {
                    return None;
                }
            }
        }
        let t = if t_near > t_min {
            t_near
        } else if t_far < t_max {
            t_far
        } else {
            return None;
        };
        let p = ray.at(t);
        let normal = {
            let mut n = Vec3::zero();
            (0..3).for_each(|a| {
                if (p[a] - self.minimum[a]).abs() < 0.0001 {
                    n[a] = -1.0;
                } else if (p[a] - self.maximum[a]).abs() < 0.0001 {
                    n[a] = 1.0;
                }
            });
            n
        };
        Some(HitRecord::new(t, p, normal.normalized(), &self.material))
    }

    fn bounding_box(&self) -> Aabb {
        Aabb {
            minimum: self.minimum,
            maximum: self.maximum,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Aabb {
    minimum: Vec3,
    maximum: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb {
            minimum: min,
            maximum: max,
        }
    }

    #[inline]
    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> bool {
        (0..3).all(|a| {
            let inv_d = ray.dir[a].recip();
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
            t1.min(t_max) > t0.max(t_min)
        })
    }

    #[inline]
    pub fn surrounding_box(box1: Aabb, box2: Aabb) -> Aabb {
        let small = Vec3::new(
            f32::min(box1.minimum.x, box2.minimum.x) - f32::EPSILON,
            f32::min(box1.minimum.y, box2.minimum.y) - f32::EPSILON,
            f32::min(box1.minimum.z, box2.minimum.z) - f32::EPSILON,
        );

        let big = Vec3::new(
            f32::max(box1.maximum.x, box2.maximum.x) + f32::EPSILON,
            f32::max(box1.maximum.y, box2.maximum.y) + f32::EPSILON,
            f32::max(box1.maximum.z, box2.maximum.z) + f32::EPSILON,
        );

        Aabb::new(small, big)
    }
}

#[derive(Clone)]
pub struct Bvh {
    aabb_box: Arc<Aabb>,
    children: (
        Option<Arc<dyn Hittable + Send + Sync>>,
        Option<Arc<dyn Hittable + Send + Sync>>,
    ),
}

impl Bvh {
    pub fn new(objects: &mut [Arc<dyn Hittable + Send + Sync>]) -> Bvh {
        let axis = 0;
        //fastrand::usize(0..3);

        objects.sort_by(|a, b| Bvh::box_compare(a.clone(), b.clone(), axis));
        let n = objects.len();
        let (left, right): (Option<Arc<_>>, Option<Arc<_>>) = match n {
            0 => (None, None),
            1 => (Some(Arc::clone(objects.first().unwrap())), None),
            2 => {
                let left = Some(Arc::clone(objects.first().unwrap()));
                let right = Some(Arc::clone(objects.last().unwrap()));
                (left, right)
            }
            // 3 => {
            //     let left = Arc::new(Bvh::new(&mut objects[0..2]));
            //     let right = Some(Arc::clone(objects.last().unwrap()));
            //     (Some(left), right)
            // }
            _ => {
                let mid = n / 2;
                let left = Arc::new(Bvh::new(&mut objects[..mid]));
                let right = Arc::new(Bvh::new(&mut objects[mid..]));
                (Some(left), Some(right))
            }
        };

        let surrounding = Arc::new(match (left.clone(), right.clone()) {
            (Some(left), None) => Aabb::surrounding_box(left.bounding_box(), left.bounding_box()),
            (Some(left), Some(right)) => {
                Aabb::surrounding_box(left.bounding_box(), right.bounding_box())
            }
            (_, _) => panic!(),
        });

        Bvh {
            aabb_box: surrounding,
            children: (left, right),
        }
    }

    #[inline]
    fn box_compare(
        a: Arc<dyn Hittable + Send + Sync>,
        b: Arc<dyn Hittable + Send + Sync>,
        axis: usize,
    ) -> Ordering {
        let center_a = (a.bounding_box().maximum - a.bounding_box().minimum) * 0.5;
        let center_b = (b.bounding_box().maximum - b.bounding_box().minimum) * 0.5;

        if center_a[axis] < center_b[axis] {
            Ordering::Less
        } else if center_a[axis] > center_b[axis] {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl Hittable for Bvh {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord<'_>> {
        if self.aabb_box.hit(ray, t_min, t_max) {
            if let Some(child_left) = self.children.0.as_ref() {
                return if let Some(child_right) = self.children.1.as_ref() {
                    if let Some(left) = child_left.hit(ray, t_min, t_max) {
                        if let Some(right) = child_right.hit(ray, t_min, left.t) {
                            Some(right)
                        } else {
                            Some(left)
                        }
                    } else {
                        child_right.hit(ray, t_min, t_max)
                    }
                } else {
                    child_left.hit(ray, t_min, t_max)
                };
            }
        }
        None
    }

    #[inline]
    fn bounding_box(&self) -> Aabb {
        *self.aabb_box
    }
}
