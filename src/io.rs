use png::ColorType::Rgb;
use png::Encoder;
use serde::{self, Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;
use std::sync::Arc;
use ultraviolet::Vec3;

use crate::hittable::{ABox, Bvh, Hittable, Sphere};
use crate::material::Material;
use crate::render::Renderer;
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
    shape: String,
    position: (f32, f32, f32),
    radius: Option<f32>,
    size: Option<(f32, f32, f32)>,
    material: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Surface {
    surface_type: String,
    albedo: (f32, f32, f32),
    roughness: Option<f32>,
    reflectance: Option<f32>,
    refractive_index: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Camera {
    position: (f32, f32, f32),
    lookat: (f32, f32, f32),
    fov: f32,
    focus_dist: f32,
    apeture: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Environment {
    // Fields for environment data
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
    file =
        File::open(scene.hdr.unwrap_or(
            r"C:\Git_Projects\Rust-Raytracer\scene\HDR\mud_road_puresky_2k.hdr".to_owned(),
        ))
        .expect("Failed to open specified file");
    let reader = BufReader::new(file);
    let image = radiant::load(reader).expect("Failed to load image data");
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    println!("loading objects & materials");
    scene.objects.into_iter().for_each(|obj| {
        let mat_obj = scene.materials.get(&obj.material).unwrap();
        let material = match mat_obj.surface_type.as_str() {
            "lambertian" => Material::lambertian(mat_obj.albedo),
            "metal" => Material::metal(
                mat_obj.albedo,
                mat_obj.roughness.unwrap_or(0.0),
                mat_obj.reflectance.unwrap_or(1.0),
            ),
            "glossy" => Material::glossy(
                mat_obj.albedo,
                mat_obj.roughness.unwrap_or(0.0),
                mat_obj.reflectance.unwrap_or(1.0),
            ),
            "dielectric" => {
                Material::dielectric(mat_obj.albedo, mat_obj.refractive_index.unwrap_or(1.52))
            }
            _ => panic!("Found material type that doesn't exist"),
        };

        match obj.shape.as_str() {
            "sphere" => {
                world.push(Arc::new(Sphere::new(
                    obj.position,
                    obj.radius.unwrap_or(1.0),
                    material,
                )));
            }
            "axis_box" => {
                world.push(Arc::new(ABox::new(
                    obj.position,
                    obj.size.unwrap_or((1.0, 1.0, 1.0)),
                    material,
                )));
            }
            _ => panic!("Found shape type that doesn't exist"),
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
        hdr: Arc::new(image),
    })
}

pub fn random_scene(lights: bool, diffuse: bool, glossy: bool, metal: bool, glass: bool) -> Bvh {
    let mut world: Vec<Arc<dyn Hittable + Send + Sync>> = vec![];
    let ground: Material = Material::lambertian((0.5, 0.5, 0.5));
    world.push(Arc::new(ABox::new(
        (-2.0, -0.5, -2.0),
        (50.0, 1.0, 50.0),
        ground,
    )));
    if lights || diffuse || glossy || metal || glass {
        for a in -11..11 {
            for b in -10..7 {
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
                            Material::glossy(
                                albedo,
                                fastrand::f32() * 0.5,
                                (fastrand::f32() * 0.5) + 1.0,
                            ),
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
                            Material::metal((albedo.0, albedo.1, albedo.2), fuzz, 5.0),
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
                            ),
                        )));
                    }
                }
            }
        }
    }

    let glass = Material::dielectric((0.1, 0.1, 0.1), 1.52);
    let gloss = Material::glossy((0.2, 0.1, 0.05), 0.2, 0.28);
    let steel = Material::metal((0.7, 0.6, 0.5), 0.01, 1.1);
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
