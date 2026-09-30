use std::f32::consts::PI;

use ultraviolet::{Rotor3, Vec3};

use crate::material::Material;

use super::primitive::{GpuPrimitiveSource, ScenePrimitive};

#[derive(Debug, Copy, Clone)]
pub struct ABox {
    pub min: Vec3,
    pub max: Vec3,
    hollow: bool,
    pub material: Material,
}

impl ABox {
    pub fn new(center: (f32, f32, f32), size: (f32, f32, f32), material: Material) -> Self {
        let hollow = size.0.min(size.1).min(size.2) < 0.0;
        let half_size = Vec3::new(size.0.abs(), size.1.abs(), size.2.abs()) * 0.5;
        let center = Vec3::from(center);
        Self {
            min: center - half_size,
            max: center + half_size,
            hollow,
            material,
        }
    }
}

impl GpuPrimitiveSource for ABox {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        append_box_triangles(
            self.min,
            self.max,
            self.hollow,
            |point| point,
            |normal| normal,
            self.material,
            primitives,
        );
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Cube {
    axis_box: ABox,
    center: Vec3,
    rotation: Rotor3,
}

impl Cube {
    pub fn new(
        center: (f32, f32, f32),
        size: (f32, f32, f32),
        rotation: (f32, f32, f32),
        material: Material,
    ) -> Self {
        Self {
            axis_box: ABox::new(center, size, material),
            center: Vec3::from(center),
            rotation: Rotor3::from_euler_angles(
                rotation.2 * PI,
                rotation.0 * PI,
                rotation.1 * PI,
            )
            .normalized(),
        }
    }
}

impl GpuPrimitiveSource for Cube {
    fn append_gpu_primitives(&self, primitives: &mut Vec<ScenePrimitive>) {
        append_box_triangles(
            self.axis_box.min,
            self.axis_box.max,
            self.axis_box.hollow,
            |point| (point - self.center).rotated_by(self.rotation) + self.center,
            |normal| normal.rotated_by(self.rotation),
            self.axis_box.material,
            primitives,
        );
    }
}

fn append_box_triangles(
    min: Vec3,
    max: Vec3,
    hollow: bool,
    transform_point: impl Fn(Vec3) -> Vec3,
    transform_normal: impl Fn(Vec3) -> Vec3,
    material: Material,
    primitives: &mut Vec<ScenePrimitive>,
) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
    .map(transform_point);
    let faces: [([usize; 4], Vec3); 6] = [
        ([1, 3, 7, 5], Vec3::unit_x()),
        ([0, 4, 6, 2], -Vec3::unit_x()),
        ([2, 6, 7, 3], Vec3::unit_y()),
        ([0, 1, 5, 4], -Vec3::unit_y()),
        ([4, 5, 7, 6], Vec3::unit_z()),
        ([0, 2, 3, 1], -Vec3::unit_z()),
    ];
    let winding = if hollow { -1.0 } else { 1.0 };

    for (indices, face_normal) in faces {
        let normal = transform_normal(face_normal * winding);
        let points = indices.map(|index| corners[index]);
        for vertices in [
            [points[0], points[1], points[2]],
            [points[0], points[2], points[3]],
        ] {
            primitives.push(ScenePrimitive::Triangle {
                vertices,
                normals: [normal; 3],
                two_sided: true,
                material,
            });
        }
    }
}