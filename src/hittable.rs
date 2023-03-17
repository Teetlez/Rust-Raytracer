use std::{cmp::Ordering, sync::Arc};

use crate::{material::Material, ray::Ray};

use ultraviolet::{Vec3, Vec4};

pub trait Hittable {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;

    fn bounding_box(&self) -> Aabb;
}

#[derive(Clone)]
pub struct HitRecord {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Arc<Material>,
}

impl HitRecord {
    pub fn new(t: f32, point: Vec3, normal: Vec3, material: Material) -> HitRecord {
        HitRecord {
            t,
            point,
            normal,
            material: (Arc::new(material)),
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
pub struct Aabb {
    min: Vec3,
    max: Vec3,
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

#[derive(Clone)]
pub struct Bvh {
    aabb_box: Arc<Aabb>,
    left: Option<Arc<BvhNode>>,
    right: Option<Arc<BvhNode>>,
}

impl Bvh {
    pub fn new(objects: &mut [Arc<dyn Hittable + Send + Sync>]) -> Bvh {
        let axis = Self::largest_axis(objects);

        objects.sort_by(|a, b| Bvh::box_compare(a.clone(), b.clone(), axis));
        let n = objects.len();
        let (left, right): (Option<Arc<BvhNode>>, Option<Arc<BvhNode>>) = match n {
            0 => panic!(),
            1 => (
                Some(Arc::new(BvhNode::Leaf((objects.first().unwrap()).clone()))),
                None,
            ),
            2 => {
                let left = Arc::new(BvhNode::Leaf(objects.first().unwrap().clone()));
                let right = Arc::new(BvhNode::Leaf(objects.last().unwrap().clone()));
                (Some(left), Some(right))
            }
            3 => {
                let left = Arc::new(BvhNode::Branch(Arc::new(Bvh::new(&mut objects[0..2]))));
                let right = Arc::new(BvhNode::Leaf(objects.last().unwrap().clone()));
                (Some(left), Some(right))
            }
            _ => {
                let mid = Self::true_middle(objects, axis);
                let left = Arc::new(BvhNode::Branch(Arc::new(Bvh::new(&mut objects[..mid]))));
                let right = Arc::new(BvhNode::Branch(Arc::new(Bvh::new(&mut objects[mid..]))));
                (Some(left), Some(right))
            }
        };

        let surrounding = Arc::new(match (left.clone(), right.clone()) {
            (Some(left), Some(right)) => {
                Aabb::surrounding_box(left.bounding_box(), right.bounding_box())
            }
            (Some(leaf), None) | (None, Some(leaf)) => leaf.bounding_box(),
            (None, None) => panic!(),
        });

        Bvh {
            aabb_box: surrounding,
            left,
            right,
        }
    }

    fn largest_axis(objects: &[Arc<dyn Hittable + Send + Sync>]) -> usize {
        if objects.is_empty() {
            return fastrand::usize(0..3);
        }
        let bbox = objects
            .iter()
            .fold(objects.first().unwrap().bounding_box(), |acc, obj| {
                Aabb::surrounding_box(acc, obj.bounding_box())
            });

        let width = bbox.max.x - bbox.min.x;
        let height = bbox.max.y - bbox.min.y;
        let depth = bbox.max.z - bbox.min.z;

        if width > height && width > depth {
            0 // X axis
        } else if height > depth {
            1 // Y axis
        } else {
            2 // Z axis
        }
    }

    fn true_middle(objects: &[Arc<dyn Hittable + Send + Sync>], axis: usize) -> usize {
        let mut mid_index = 1;
        let mid_world = (objects.first().unwrap().bounding_box().min[axis]
            + objects.last().unwrap().bounding_box().max[axis])
            * 0.5;
        while mid_index < objects.len() - 1
            && objects[mid_index].bounding_box().center()[axis] <= mid_world
        {
            mid_index += 1;
        }
        mid_index
    }

    #[inline]
    fn box_compare(
        a: Arc<dyn Hittable + Send + Sync>,
        b: Arc<dyn Hittable + Send + Sync>,
        axis: usize,
    ) -> Ordering {
        let center_a = a.bounding_box().max + a.bounding_box().min * 0.5;
        let center_b = b.bounding_box().max + b.bounding_box().min * 0.5;

        if a.bounding_box().surrounds_axis(b.bounding_box(), axis) {
            Ordering::Less
        } else if b.bounding_box().surrounds_axis(a.bounding_box(), axis) {
            Ordering::Greater
        } else if center_a[axis] < center_b[axis] {
            Ordering::Less
        } else if center_a[axis] > center_b[axis] {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

enum BvhNode {
    Branch(Arc<Bvh>),
    Leaf(Arc<dyn Hittable + Send + Sync>),
}

impl Hittable for BvhNode {
    fn bounding_box(&self) -> Aabb {
        match self {
            BvhNode::Branch(branch) => branch.bounding_box(),
            BvhNode::Leaf(leaf) => leaf.bounding_box(),
        }
    }

    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        match self {
            BvhNode::Branch(branch) => branch.hit(ray, t_min, t_max),
            BvhNode::Leaf(leaf) => leaf.hit(ray, t_min, t_max),
        }
    }
}

impl Hittable for Bvh {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        if self
            .aabb_box
            .hit(ray.pos, ray.dir.map(|k| k.recip()), t_min, t_max)
        {
            if let Some(child_left) = self.left.as_ref() {
                return if let Some(child_right) = self.right.as_ref() {
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
    // fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
    //     let mut stack = vec![BvhNode::Branch(Arc::new(self.clone()))];

    //     let mut closest_hit: Option<HitRecord> = None;
    //     let mut closest_t = t_max;

    //     let inv_d = ray.dir.map(|k| k.recip());

    //     while let Some(node) = stack.pop() {
    //         match node {
    //             BvhNode::Branch(bvh) => {
    //                 if bvh.aabb_box.hit(ray.pos, inv_d, t_min, t_max) {
    //                     if let Some(left_child) = &bvh.left {
    //                         stack.push(BvhNode::Leaf(left_child.clone()));
    //                     }
    //                     if let Some(right_child) = &bvh.right {
    //                         stack.push(BvhNode::Leaf(right_child.clone()));
    //                     }
    //                 }
    //             }
    //             BvhNode::Leaf(hittable) => {
    //                 if let Some(hit) = hittable.hit(ray, t_min, closest_t) {
    //                     if hit.t < closest_t {
    //                         closest_t = hit.t;
    //                         closest_hit = Some(hit);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     closest_hit
    // }

    #[inline]
    fn bounding_box(&self) -> Aabb {
        *self.aabb_box
    }
}
