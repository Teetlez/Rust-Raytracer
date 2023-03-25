use std::{ops::Mul, sync::Arc};

use crate::{material::Material, ray::Ray};

use ultraviolet::{Rotor3, Vec3};

use super::{
    bvh::Bvh,
    cube::Aabb,
    hittable::{HitRecord, Hittable},
    triangle::Triangle,
};

#[derive(Clone)]
pub struct Mesh {
    pub bvh: Bvh,
    pub material: Arc<Material>,
    pub cull_backface: bool,
}

impl Mesh {
    pub fn new(
        polygons: &tobj::Mesh,
        translation: Vec3,
        scale: Vec3,
        rotation: Vec3,
        cull_backface: bool,
        material: Material,
    ) -> Mesh {
        let mut mesh: Vec<Arc<dyn Hittable + Send + Sync>> = Vec::new();
        let mat_ref = Arc::new(material);
        let rot = Rotor3::from_euler_angles(rotation.z, rotation.x, rotation.y).normalized();
        polygons.indices.chunks_exact(3).for_each(|face| {
            let vertices: [Vec3; 3] = [
                Vec3::new(
                    polygons.positions[(3 * face[0] as usize)],
                    polygons.positions[(3 * face[0] as usize) + 1],
                    polygons.positions[(3 * face[0] as usize) + 2],
                ),
                Vec3::new(
                    polygons.positions[(3 * face[1] as usize)],
                    polygons.positions[(3 * face[1] as usize) + 1],
                    polygons.positions[(3 * face[1] as usize) + 2],
                ),
                Vec3::new(
                    polygons.positions[(3 * face[2] as usize)],
                    polygons.positions[(3 * face[2] as usize) + 1],
                    polygons.positions[(3 * face[2] as usize) + 2],
                ),
            ]
            .into_iter()
            .map(|vertex| vertex.mul(scale).rotated_by(rot) + translation)
            .collect::<Vec<Vec3>>()
            .try_into()
            .unwrap();

            let mut normals: [Vec3; 3] = [
                Vec3::new(
                    polygons.normals[(3 * face[0] as usize)],
                    polygons.normals[(3 * face[0] as usize) + 1],
                    polygons.normals[(3 * face[0] as usize) + 2],
                ),
                Vec3::new(
                    polygons.normals[(3 * face[1] as usize)],
                    polygons.normals[(3 * face[1] as usize) + 1],
                    polygons.normals[(3 * face[1] as usize) + 2],
                ),
                Vec3::new(
                    polygons.normals[(3 * face[2] as usize)],
                    polygons.normals[(3 * face[2] as usize) + 1],
                    polygons.normals[(3 * face[2] as usize) + 2],
                ),
            ];
            rot.rotate_vecs(&mut normals);

            mesh.push(Arc::new(Triangle::new(
                vertices,
                normals,
                !cull_backface,
                mat_ref.clone(),
            )));
        });
        Mesh {
            bvh: Bvh::new(mesh.as_mut_slice()),
            material: mat_ref,
            cull_backface,
        }
    }
}

impl Hittable for Mesh {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        self.bvh.hit(ray, t_min, t_max)
    }

    fn bounding_box(&self) -> Aabb {
        *self.bvh.aabb_box
    }
}
