use png::{ColorType::Rgb, Encoder};
use serde::{self, Deserialize, Serialize};
use std::{
    f32::consts::PI,
    fs::File,
    io::{BufReader, BufWriter, Read},
    path::Path,
    sync::Arc,
};
use ultraviolet::Vec3;

use crate::material::Material;
use crate::render::Renderer;
use crate::tracer::{
    bvh::Bvh,
    cube::{ABox, Cube},
    hittable::Hittable,
    mesh::Mesh,
    sphere::Sphere,
    triangle::Triangle,
};
use crate::{camera, Args};

#[derive(Debug, Deserialize, Serialize)]
struct Scene {
    hdr: Option<String>,
    camera: Camera,
    materials: std::collections::HashMap<String, Surface>,
    objects: Vec<Object>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Object {
    name: Option<String>,
    shape: Shape,
    material: String,
}

#[derive(Debug, Deserialize, Serialize)]
enum Shape {
    Sphere(
        (f32, f32, f32), // position
        Option<f32>,     // raduis
    ),
    Triangle(
        ([f32; 3], [f32; 3], [f32; 3]), // vertices
    ),
    Box(
        (f32, f32, f32),         // position
        Option<(f32, f32, f32)>, // size
        Option<(f32, f32, f32)>, // rotation
    ),
    AxisBox(
        (f32, f32, f32),         // position
        Option<(f32, f32, f32)>, // size
    ),
    Mesh(
        String,                  // file path
        Option<(f32, f32, f32)>, // translation
        Option<(f32, f32, f32)>, // scale
        Option<(f32, f32, f32)>, // rotation
        bool,                    // cull backface
    ),
}

#[derive(Debug, Deserialize, Serialize)]
enum Surface {
    Lambertian(
        (f32, f32, f32), // albedo
    ),
    Glossy(
        (f32, f32, f32), // albedo
        Option<f32>,     // roughness
        Option<f32>,     // reflectence
    ),
    Metal(
        (f32, f32, f32), // albedo
        Option<f32>,     // roughness
    ),
    Dielectric(
        (f32, f32, f32), // absorption
        Option<f32>,     // refractive_index
        Option<f32>,     // roughness
    ),
}

#[derive(Debug, Deserialize, Serialize)]
struct Camera {
    position: (f32, f32, f32),
    lookat: (f32, f32, f32),
    fov: f32,
    focus_dist: f32,
    apeture: f32,
}

pub fn load_scene(scene_file: &Path, args: &Args) -> Result<Renderer, Box<dyn std::error::Error>> {
    println!("reading file");
    let mut file = File::open(scene_file)?;
    let mut contents = String::new();
    println!("loading contents");
    file.read_to_string(&mut contents)?;
    println!("parsing contents");
    let scene: Scene = ron::de::from_str(&contents)?;

    println!("loading hdr");
    let image = if let Ok(f) = File::open(scene.hdr.unwrap_or("".to_string())) {
        let reader = BufReader::new(f);
        Arc::new(radiant::load(reader).ok())
    } else {
        Arc::new(None)
    };
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    println!("loading objects & materials");
    scene.objects.into_iter().for_each(|obj| {
        let material = match *scene.materials.get(&obj.material).unwrap() {
            Surface::Lambertian(albedo) => Material::lambertian(albedo),
            Surface::Metal(albedo, roughness) => Material::metal(albedo, roughness.unwrap_or(0.0)),
            Surface::Glossy(albedo, reflectance, roughness) => {
                Material::glossy(albedo, reflectance.unwrap_or(1.0), roughness.unwrap_or(0.0))
            }
            Surface::Dielectric(absorption, refractive_index, roughness) => Material::dielectric(
                absorption,
                refractive_index.unwrap_or(1.52),
                roughness.unwrap_or(0.0),
            ),
        };

        match obj.shape {
            Shape::Sphere(position, radius) => {
                world.push(Arc::new(Sphere::new(
                    position,
                    radius.unwrap_or(1.0),
                    material,
                )));
            }
            Shape::Triangle(vertices) => {
                let vertices = [
                    Vec3::from(vertices.0),
                    Vec3::from(vertices.1),
                    Vec3::from(vertices.2),
                ];
                let normal = (vertices[1] - vertices[0]).cross(vertices[2] - vertices[0]);
                world.push(Arc::new(Triangle::new(
                    vertices,
                    [normal; 3],
                    true,
                    material,
                )));
            }
            Shape::Box(position, size, rotation) => world.push(Arc::new(Cube::new(
                position,
                size.unwrap_or((1.0, 1.0, 1.0)),
                rotation.unwrap_or((0.0, 0.0, 0.0)),
                material,
            ))),
            Shape::AxisBox(position, size) => {
                world.push(Arc::new(ABox::new(
                    position,
                    size.unwrap_or((1.0, 1.0, 1.0)),
                    material,
                )));
            }
            Shape::Mesh(location, translation, scale, rotation, cull_backface) => {
                let (models, _) = tobj::load_obj(
                    location,
                    &tobj::LoadOptions {
                        single_index: true,
                        triangulate: true,
                        ignore_points: true,
                        ignore_lines: true,
                    },
                )
                .expect("failed to load file");
                let mut meshes: Vec<Arc<dyn Hittable + Send + Sync>> = Vec::new();
                models.iter().for_each(|model| {
                    meshes.push(Arc::new(Mesh::new(
                        &model.mesh,
                        Vec3::from(translation.unwrap_or((0.0, 0.0, 0.0))),
                        Vec3::from(scale.unwrap_or((1.0, 1.0, 1.0))),
                        Vec3::from(rotation.unwrap_or((0.0, 0.0, 0.0))) * PI,
                        cull_backface,
                        material,
                    )));
                });
                world.append(&mut meshes);
            }
        }
    });
    println!("building BVH");
    let bvh = Bvh::new(&mut world);

    Ok(Renderer {
        width: args.width,
        height: args.height,
        camera: camera::Camera::new(
            Vec3::from(scene.camera.position),
            Vec3::from(scene.camera.lookat),
            Vec3::unit_y(),
            scene.camera.fov,
            args.width as f32 / args.height as f32,
            scene.camera.apeture,
            scene.camera.focus_dist,
        ),
        world: Arc::new(bvh),
        sample_rate: args.samples,
        max_bounce: args.bounces,
        hdr: image,
        light_clamp: args.light_clamp,
    })
}

pub fn random_scene(lights: bool, diffuse: bool, glossy: bool, metal: bool, glass: bool) -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    let ground: Material = Material::glossy((0.55, 0.53, 0.56), 0.1, 0.7);
    world.push(Arc::new(ABox::new(
        (-2.0, -0.5, -2.0),
        (50.0, 1.0, 50.0),
        ground,
    )));
    if lights || diffuse || glossy || metal || glass {
        for a in -11..11 {
            for b in -11..11 {
                let choose_mat = fastrand::f32();
                let center = (
                    a as f32 + 0.9 * fastrand::f32(),
                    0.2,
                    b as f32 + 0.9 * fastrand::f32(),
                );

                if (Vec3::new(center.0, center.1, center.2) - Vec3::new(4.0, 0.2, 0.0)).mag() > 0.9
                {
                    let albedo = (
                        fastrand::f32() * fastrand::f32(),
                        fastrand::f32() * fastrand::f32(),
                        fastrand::f32() * fastrand::f32(),
                    );
                    if glossy && choose_mat < 0.3 {
                        // glossy
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::glossy(albedo, fastrand::f32() + 0.5, fastrand::f32() * 0.5),
                        )));
                    } else if diffuse && choose_mat < 0.6 {
                        // diffuse
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::lambertian(albedo),
                        )));
                    } else if metal && choose_mat < 0.8 {
                        // metal
                        let fuzz = 0.5 * fastrand::f32();
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::metal((albedo.0, albedo.1, albedo.2), fuzz),
                        )));
                    } else if lights && choose_mat < 0.9 {
                        // lights
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::lambertian((
                                fastrand::f32() * 6.0,
                                fastrand::f32() * 6.0,
                                fastrand::f32() * 6.0,
                            )),
                        )));
                    } else if glass {
                        // glass
                        world.push(Arc::new(Sphere::new(
                            center,
                            0.2,
                            Material::dielectric(
                                (fastrand::f32(), fastrand::f32(), fastrand::f32()),
                                1.52,
                                fastrand::f32() * 0.5,
                            ),
                        )));
                    }
                }
            }
        }
    }

    let glass = Material::dielectric((0.1, 0.1, 0.1), 1.52, 0.025);
    let gloss = Material::glossy((0.3, 0.2, 0.15), 0.6, 0.3);
    let steel = Material::metal((0.7, 0.5, 0.3), 0.025);
    // let diffuse = Material::lambertian((0.4, 0.2, 0.1));

    world.push(Arc::new(Sphere::new((4.0, 1.0, 0.0), 1.0, steel)));
    world.push(Arc::new(Sphere::new((0.0, 1.0, 0.0), 1.0, glass)));
    world.push(Arc::new(Sphere::new((-4.0, 1.0, 0.0), 1.0, gloss)));
    // world.push(Arc::new(Sphere::new((-4.5, 1.0, 0.0), 1.0, diffuse)));

    Bvh::new(&mut world)
}

// Define a helper function to convert a 32-bit int color to an RGB triplet
fn int_to_rgb(color: u32) -> [u8; 3] {
    [
        ((color & 0x00ff0000) >> 16) as u8,
        ((color & 0x0000ff00) >> 8) as u8,
        (color & 0x000000ff) as u8,
    ]
}

// Define the function to save the colors as an image
pub fn save_colors_as_image(
    colors: &[u32],
    width: u32,
    height: u32,
    filename: &str,
) -> std::io::Result<()> {
    // Create a new PNG encoder with the specified width and height
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    let mut encoder = Encoder::new(&mut writer, width, height);

    // Set the color type of the PNG to RGB and configure the encoder
    encoder.set_color(Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Best);
    encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);

    // Encode and write the image data
    let mut writer = encoder.write_header()?;
    let mut data = Vec::new();
    for color in colors {
        data.extend_from_slice(&int_to_rgb(*color));
    }
    writer.write_image_data(&data)?;

    Ok(())
}
