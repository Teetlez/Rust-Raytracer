use std::ops::Mul;

use crate::material::Material;

use ultraviolet::{Rotor3, Vec3};

use super::{
    primitive::{GpuPrimitiveSource, ScenePrimitive},
    triangle::Triangle,
};

#[derive(Clone)]
pub struct Mesh {
    triangles: Vec<Triangle>,
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
        let mut triangles = Vec::with_capacity(polygons.indices.len() / 3);
        let rot = Rotor3::from_euler_angles(rotation.z, rotation.x, rotation.y).normalized();
        for face in polygons.indices.as_chunks::<3>().0 {
            let vertices = face.map(|index| {
                Vec3::new(
                    polygons.positions[3 * index as usize],
                    polygons.positions[(3 * index as usize) + 1],
                    polygons.positions[(3 * index as usize) + 2],
                )
                .mul(scale)
                .rotated_by(rot)
                    + translation
            });
            let mut normals = face.map(|index| {
                Vec3::new(
                    polygons.normals[3 * index as usize],
                    polygons.normals[(3 * index as usize) + 1],
                    polygons.normals[(3 * index as usize) + 2],
                )
            });
            rot.rotate_vecs(&mut normals);

            triangles.push(Triangle::new(vertices, normals, !cull_backface, material));
        }
        Mesh { triangles }
    }
}

impl GpuPrimitiveSource for Mesh {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        for triangle in &self.triangles {
            triangle.append_gpu_primitives(primitives);
        }
    }
}
