use bytemuck::{Pod, Zeroable};
use ultraviolet::Vec3;

use crate::{material::Material, tracer::primitive::ScenePrimitive};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuPrimitive {
    pub data: [u32; 4],
    pub p0: [f32; 4],
    pub p1: [f32; 4],
    pub p2: [f32; 4],
    pub n0: [f32; 4],
    pub n1: [f32; 4],
    pub n2: [f32; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuMaterial {
    pub color: [f32; 4],
    pub params: [f32; 4],
    pub info: [u32; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuBvhNode {
    pub min: [f32; 4],
    pub max: [f32; 4],
    pub links: [u32; 4],
}

pub struct GpuScene {
    pub primitives: Vec<GpuPrimitive>,
    pub materials: Vec<GpuMaterial>,
    pub nodes: Vec<GpuBvhNode>,
}

#[derive(Clone, Copy)]
struct BoundedPrimitive {
    primitive: GpuPrimitive,
    min: Vec3,
    max: Vec3,
}

impl GpuScene {
    pub fn from_primitives(primitives: Vec<ScenePrimitive>) -> Self {
        let mut materials = Vec::with_capacity(primitives.len());
        let mut bounded = primitives
            .into_iter()
            .map(|primitive| {
                let (mut packed, material, min, max) = pack_primitive(primitive);
                packed.data[1] = materials.len() as u32;
                materials.push(pack_material(material));
                BoundedPrimitive {
                    primitive: packed,
                    min,
                    max,
                }
            })
            .collect::<Vec<_>>();

        let mut packed_primitives = Vec::with_capacity(bounded.len().max(1));
        let mut nodes = Vec::with_capacity((bounded.len() * 2).max(1));
        if bounded.is_empty() {
            let mut empty_leaf = GpuBvhNode::zeroed();
            empty_leaf.links[0] = u32::MAX;
            empty_leaf.links[1] = u32::MAX;
            nodes.push(empty_leaf);
        } else {
            build_bvh(&mut bounded, &mut packed_primitives, &mut nodes);
        }
        let escape_index = nodes.len() as u32;
        set_escape_links(&mut nodes, 0, escape_index);

        Self {
            primitives: packed_primitives,
            materials,
            nodes,
        }
    }
}

fn pack_primitive(primitive: ScenePrimitive) -> (GpuPrimitive, Material, Vec3, Vec3) {
    match primitive {
        ScenePrimitive::Sphere {
            center,
            radius,
            material,
        } => {
            let radius = radius.abs();
            let mut packed = GpuPrimitive::zeroed();
            packed.data[0] = 0;
            packed.p0 = vec4_with_w(center, radius);
            let extent = Vec3::one() * radius;
            (packed, material, center - extent, center + extent)
        }
        ScenePrimitive::Triangle {
            vertices,
            normals,
            two_sided,
            material,
        } => {
            let mut packed = GpuPrimitive::zeroed();
            packed.data[0] = 1;
            packed.data[2] = u32::from(two_sided);
            packed.p0 = vec4(vertices[0]);
            packed.p1 = vec4(vertices[1]);
            packed.p2 = vec4(vertices[2]);
            packed.n0 = vec4(normals[0]);
            packed.n1 = vec4(normals[1]);
            packed.n2 = vec4(normals[2]);
            let padding = Vec3::one() * 0.0001;
            let min = vertices[0]
                .min_by_component(vertices[1])
                .min_by_component(vertices[2])
                - padding;
            let max = vertices[0]
                .max_by_component(vertices[1])
                .max_by_component(vertices[2])
                + padding;
            (packed, material, min, max)
        }
    }
}

fn pack_material(material: Material) -> GpuMaterial {
    let (color, kind, roughness, reflectance, ior) = match material {
        Material::Lambertian(value) => (value.albedo, 0, 0.0, 0.0, 1.0),
        Material::Glossy(value) => (value.albedo, 1, value.roughness, value.reflectance, 1.0),
        Material::Metal(value) => (value.albedo, 2, value.roughness, 0.0, 1.0),
        Material::Dielectric(value) => (
            value.albedo,
            3,
            value.roughness,
            0.0,
            value.refractive_index,
        ),
    };
    GpuMaterial {
        color: [color.x, color.y, color.z, 0.0],
        params: [roughness, reflectance, ior, 0.0],
        info: [kind, 0, 0, 0],
    }
}

fn build_bvh(
    bounded: &mut [BoundedPrimitive],
    packed_primitives: &mut Vec<GpuPrimitive>,
    nodes: &mut Vec<GpuBvhNode>,
) -> u32 {
    let min = bounded.iter().fold(
        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        |acc, primitive| acc.min_by_component(primitive.min),
    );
    let max = bounded.iter().fold(
        Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        |acc, primitive| acc.max_by_component(primitive.max),
    );
    let node_index = nodes.len() as u32;
    nodes.push(GpuBvhNode::zeroed());

    if bounded.len() <= 4 {
        let start = packed_primitives.len() as u32;
        packed_primitives.extend(bounded.iter().map(|item| item.primitive));
        nodes[node_index as usize] = GpuBvhNode {
            min: vec4(min),
            max: vec4(max),
            links: [u32::MAX, u32::MAX, start, bounded.len() as u32],
        };
        return node_index;
    }

    let extent = max - min;
    let axis = if extent.x > extent.y && extent.x > extent.z {
        0
    } else if extent.y > extent.z {
        1
    } else {
        2
    };
    bounded.sort_by(|a, b| {
        let center_a = (a.min[axis] + a.max[axis]) * 0.5;
        let center_b = (b.min[axis] + b.max[axis]) * 0.5;
        center_a.total_cmp(&center_b)
    });
    let midpoint = bounded.len() / 2;
    let (left_items, right_items) = bounded.split_at_mut(midpoint);
    let left = build_bvh(left_items, packed_primitives, nodes);
    let right = build_bvh(right_items, packed_primitives, nodes);
    nodes[node_index as usize] = GpuBvhNode {
        min: vec4(min),
        max: vec4(max),
        links: [left, right, 0, 0],
    };
    node_index
}

fn set_escape_links(nodes: &mut [GpuBvhNode], node_index: u32, escape_index: u32) {
    let node = &mut nodes[node_index as usize];
    if node.links[0] == u32::MAX {
        node.links[1] = escape_index;
        return;
    }
    let left = node.links[0];
    let right = node.links[1];
    node.links[2] = escape_index;
    set_escape_links(nodes, left, right);
    set_escape_links(nodes, right, escape_index);
}

fn vec4(value: Vec3) -> [f32; 4] {
    [value.x, value.y, value.z, 0.0]
}

fn vec4_with_w(value: Vec3, w: f32) -> [f32; 4] {
    [value.x, value.y, value.z, w]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattened_bvh_covers_every_primitive_once() {
        let primitives = (0..9)
            .map(|index| ScenePrimitive::Sphere {
                center: Vec3::new(index as f32, 0.0, 0.0),
                radius: 0.5,
                material: Material::lambertian((0.5, 0.5, 0.5)),
            })
            .collect();
        let scene = GpuScene::from_primitives(primitives);

        assert_eq!(scene.primitives.len(), 9);
        assert_eq!(scene.materials.len(), 9);
        let leaf_primitive_count = scene
            .nodes
            .iter()
            .filter(|node| node.links[0] == u32::MAX)
            .map(|node| node.links[3] as usize)
            .sum::<usize>();
        assert_eq!(leaf_primitive_count, 9);
        assert!(scene.nodes.len() > 1);
    }

    #[test]
    fn stackless_escape_links_visit_nodes_in_preorder() {
        let primitives = (0..17)
            .map(|index| ScenePrimitive::Sphere {
                center: Vec3::new(index as f32, 0.0, 0.0),
                radius: 0.5,
                material: Material::lambertian((0.5, 0.5, 0.5)),
            })
            .collect();
        let scene = GpuScene::from_primitives(primitives);

        let mut visited = Vec::new();
        let mut node_index = 0;
        while (node_index as usize) < scene.nodes.len() {
            visited.push(node_index as usize);
            let node = scene.nodes[node_index as usize];
            node_index = if node.links[0] == u32::MAX {
                node.links[1]
            } else {
                node.links[0]
            };
        }

        assert_eq!(visited, (0..scene.nodes.len()).collect::<Vec<_>>());
        assert_eq!(node_index as usize, scene.nodes.len());
    }

    #[test]
    fn empty_bvh_has_a_terminating_leaf() {
        let scene = GpuScene::from_primitives(Vec::new());

        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.nodes[0].links[0], u32::MAX);
        assert!(scene.primitives.is_empty());
    }
}
