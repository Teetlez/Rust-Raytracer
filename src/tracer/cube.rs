use std::f32::consts::PI;

use ultraviolet::{Rotor3, Vec3};

use crate::{material::Material, ray::Ray};

use super::hittable::{HitRecord, Hittable};

#[derive(Debug, Copy, Clone)]
pub struct ABox {
    pub min: Vec3,
    pub max: Vec3,
    hollow: bool,
    pub material: Material,
}

impl ABox {
    pub fn new(center: (f32, f32, f32), size: (f32, f32, f32), mat: Material) -> ABox {
        let hollow = size.0.min(size.1).min(size.2) < 0.0;
        let minimum = Vec3::new(
            center.0 - (size.0 * 0.5).abs(),
            center.1 - (size.1 * 0.5).abs(),
            center.2 - (size.2 * 0.5).abs(),
        );
        let maximum = Vec3::new(
            center.0 + (size.0 * 0.5).abs(),
            center.1 + (size.1 * 0.5).abs(),
            center.2 + (size.2 * 0.5).abs(),
        );
        ABox {
            min: minimum,
            max: maximum,
            hollow,
            material: mat,
        }
    }
}

impl Hittable for ABox {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let inv_d = ray.dir.map(|k| k.recip());
        let t0 = (self.min - ray.pos) * inv_d;
        let t1 = (self.max - ray.pos) * inv_d;

        let t_near = t0.min_by_component(t1).component_max().max(t_min);
        let t_far = t0.max_by_component(t1).component_min().min(t_max);
        if t_near <= t_far {
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
                    if (p[a] - self.min[a]).abs() < 0.0001 {
                        n[a] = -1.0;
                    } else if (p[a] - self.max[a]).abs() < 0.0001 {
                        n[a] = 1.0;
                    }
                });
                if self.hollow {
                    -n
                } else {
                    n
                }
            };
            Some(HitRecord::new(t, p, normal.normalized(), self.material))
        } else {
            None
        }
    }

    fn bounding_box(&self) -> Aabb {
        Aabb {
            min: self.min,
            max: self.max,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Cube {
    pub axis_box: ABox,
    center: Vec3,
    pub rotation: Rotor3,
}

impl Cube {
    pub fn new(
        center: (f32, f32, f32),
        size: (f32, f32, f32),
        rotation: (f32, f32, f32),
        mat: Material,
    ) -> Cube {
        Cube {
            axis_box: ABox::new(center, size, mat),
            center: Vec3::from(center),
            rotation: Rotor3::from_euler_angles(
                rotation.2 * (PI / 2.0),
                rotation.0 * (PI / 2.0),
                rotation.1 * (PI / 2.0),
            )
            .normalized(),
        }
    }
}

impl Hittable for Cube {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let rot_pos = (ray.pos - self.center).rotated_by(self.rotation.reversed()) + self.center;
        let rot_dir = ray.dir.rotated_by(self.rotation.reversed());

        if let Some(hit) = self.axis_box.hit(&Ray::new(rot_pos, rot_dir), t_min, t_max) {
            Some(HitRecord {
                t: hit.t,
                point: ray.at(hit.t),
                normal: hit.normal.rotated_by(self.rotation),
                material: hit.material,
            })
        } else {
            None
        }
    }

    fn bounding_box(&self) -> Aabb {
        let bbox = self.axis_box.bounding_box();
        let vec_box = [
            Vec3::new(bbox.min.x, bbox.min.y, bbox.min.z),
            Vec3::new(bbox.max.x, bbox.min.y, bbox.min.z),
            Vec3::new(bbox.min.x, bbox.max.y, bbox.min.z),
            Vec3::new(bbox.max.x, bbox.max.y, bbox.min.z),
            Vec3::new(bbox.min.x, bbox.min.y, bbox.max.z),
            Vec3::new(bbox.max.x, bbox.min.y, bbox.max.z),
            Vec3::new(bbox.min.x, bbox.max.y, bbox.max.z),
            Vec3::new(bbox.max.x, bbox.max.y, bbox.max.z),
        ];
        let transformed_vecs = vec_box
            .iter()
            .map(|vec| (*vec - self.center).rotated_by(self.rotation) + self.center);
        let (min, max) = transformed_vecs.fold(
            (
                Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
            ),
            |(min, max), vec| {
                (
                    Vec3::new(min.x.min(vec.x), min.y.min(vec.y), min.z.min(vec.z)),
                    Vec3::new(max.x.max(vec.x), max.y.max(vec.y), max.z.max(vec.z)),
                )
            },
        );
        Aabb { min, max }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }

    #[inline]
    pub fn hit(&self, origin: Vec3, inv_d: Vec3, t_min: f32, t_max: f32) -> bool {
        let t0 = (self.min - origin) * inv_d;
        let t1 = (self.max - origin) * inv_d;

        let t_near = t0.min_by_component(t1);
        let t_far = t0.max_by_component(t1);

        t_near.component_max().max(t_min) <= t_far.component_min().min(t_max)
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.max + self.min) * 0.5
    }

    #[inline]
    pub fn surrounding_box(box1: Aabb, box2: Aabb) -> Aabb {
        let min = Vec3::new(
            f32::min(box1.min.x, box2.min.x) - f32::EPSILON,
            f32::min(box1.min.y, box2.min.y) - f32::EPSILON,
            f32::min(box1.min.z, box2.min.z) - f32::EPSILON,
        );

        let max = Vec3::new(
            f32::max(box1.max.x, box2.max.x) + f32::EPSILON,
            f32::max(box1.max.y, box2.max.y) + f32::EPSILON,
            f32::max(box1.max.z, box2.max.z) + f32::EPSILON,
        );

        Aabb::new(min, max)
    }

    #[inline]
    pub fn surrounds_axis(&self, other: Aabb, axis: usize) -> bool {
        self.min[axis] < other.min[axis] && self.max[axis] > other.max[axis]
    }
}
