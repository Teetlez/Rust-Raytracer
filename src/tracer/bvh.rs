use std::{cmp::Ordering, sync::Arc};

use crate::ray::Ray;

use super::{
    cube::Aabb,
    hittable::{HitRecord, Hittable},
};

pub enum BvhNode {
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

#[derive(Clone)]
pub struct Bvh {
    pub aabb_box: Arc<Aabb>,
    pub left: Option<Arc<BvhNode>>,
    pub right: Option<Arc<BvhNode>>,
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

impl Hittable for Bvh {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        if self
            .aabb_box
            .hit(ray.pos, ray.dir.map(|k| k.recip()), t_min, t_max)
        {
            let right_hit = self
                .right
                .as_ref()
                .and_then(|child_right| child_right.hit(ray, t_min, t_max));

            let left_hit = self.left.as_ref().and_then(|child_left| {
                child_left.hit(ray, t_min, right_hit.map_or(t_max, |hit| hit.t))
            });

            left_hit.or(right_hit)
        } else {
            None
        }
    }

    #[inline]
    fn bounding_box(&self) -> Aabb {
        *self.aabb_box
    }
}
