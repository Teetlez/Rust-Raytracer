use std::sync::Arc;

use crate::{material::Material, ray::Ray};

use ultraviolet::Vec3;

use super::{
    cube::Aabb,
    hittable::{HitRecord, Hittable},
};

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
    pub material: Arc<Material>,
    two_sided: bool,
}

impl Triangle {
    pub fn new(verticies: [Vec3; 3], two_sided: bool, material: Arc<Material>) -> Triangle {
        Triangle {
            vertices: verticies,
            normal: (verticies[1] - verticies[0])
                .cross(verticies[2] - verticies[0])
                .normalized(),
            material,
            two_sided,
        }
    }
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        if self.two_sided || ray.dir.dot(self.normal) < 0.0 {
            let edge1 = self.vertices[1] - self.vertices[0];
            let edge2 = self.vertices[2] - self.vertices[0];
            let h = ray.dir.cross(edge2);
            let a = edge1.dot(h);

            if (-1e-6..=1e-6).contains(&a) {
                return None;
            }

            let f = 1.0 / a;
            let s = ray.pos - self.vertices[0];
            let u = f * s.dot(h);

            if !(0.0..=1.0).contains(&u) {
                return None;
            }

            let q = s.cross(edge1);
            let v = f * ray.dir.dot(q);

            if v < 0.0 || u + v > 1.0 {
                return None;
            }

            let t = f * edge2.dot(q);

            if !(t_min..=t_max).contains(&t) {
                return None;
            }

            let mut normal = self.normal;

            if self.two_sided && ray.dir.dot(normal) > 0.0 {
                normal = -normal;
            }

            let front_face = ray.dir.dot(normal) < 0.0;
            let normal = if front_face { normal } else { -normal };

            Some(HitRecord {
                t,
                point: ray.at(t),
                normal,
                material: self.material.clone(),
            })
        } else {
            None
        }
    }

    fn bounding_box(&self) -> Aabb {
        Aabb {
            min: Vec3::new(
                self.vertices[0]
                    .x
                    .min(self.vertices[1].x.min(self.vertices[2].x)),
                self.vertices[0]
                    .y
                    .min(self.vertices[1].y.min(self.vertices[2].y)),
                self.vertices[0]
                    .z
                    .min(self.vertices[1].z.min(self.vertices[2].z)),
            ),
            max: Vec3::new(
                self.vertices[0]
                    .x
                    .max(self.vertices[1].x.max(self.vertices[2].x)),
                self.vertices[0]
                    .y
                    .max(self.vertices[1].y.max(self.vertices[2].y)),
                self.vertices[0]
                    .z
                    .max(self.vertices[1].z.max(self.vertices[2].z)),
            ),
        }
    }
}
