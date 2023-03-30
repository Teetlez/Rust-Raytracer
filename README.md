# Rust-Raytracer

Small, personal pathtracer project written in rust.

![1678228930342](https://user-images.githubusercontent.com/47290124/227623254-7356f1e7-d0f6-4875-bd25-382a73d2fb8a.png)

## Features

- Support for lambertian, glossy, metallic, and dielectric materials
- Support for spheres, rectangles, triangles, and .obj files
- Customizable settings via command line
- Initial preview window before rendering
- Saving final render to png
- Custom scences via a .ron config file
- HDR environment lighting
- ACES tonemapping
- BVH scene optimization

## Planned Features

- [ ] Camera movement
- [ ] Volumes
- [X] Mesh smooth shading
- [ ] Cylinder object
- [ ] Planars
- [ ] BSDF
- [ ] Textures + normal maps
- [ ] GPU support
- [ ] Next Event Estimation
- [ ] Denoising

## Usage
```
rust_raytracer.exe [OPTIONS] [SCENE]

Arguments:
  [SCENE]  Scene file to use

Options:
  -s, --samples <SAMPLES>          Number of samples per pixel [default: 128]
  -p, --passes <PASSES>            Number of frames to cumulate [default: 64]
  -b, --bounces <BOUNCES>          Max number of times a ray can bounce [default: 8]
      --width <WIDTH>              Pixel width of frame [default: 640]
      --height <HEIGHT>            Pixel hight of frame [default: 480]
  -g, --gamma <GAMMA>              Gamma level [default: 2.2]
  -l, --light-clamp <LIGHT_CLAMP>  Max light brightness [default: inf]
  -f, --filter                     apply bilateral filter after render to reduce noise
  -h, --help                       Print help
  -V, --version                    Print version
```
### Example scene file

```
Scene(
    hdr: Some(".\\scene\\HDR\\studio_small_08_2k.hdr"),
    camera: (
        position: (20.0, 10.0, -20.0),
        lookat: (0.0, -0.25, 0.0),
        fov: 12.0,
        focus_dist: 30.0,
        apeture: 1.5,
    ),
    materials: {
        "steel": Metal(
            (0.7, 0.7, 0.7),  // albedo
            Some(0.05),       // roughness
        ),
        "glossy": Glossy(
            (0.7, 0.7, 0.7),  // albedo
            Some(1.5),        // reflectance
            Some(0.05),       // roughness
        ),
        "glass": Dielectric(
            (0.6, 0.1, 0.25), // absorption
            Some(1.52),       // refractive index
            Some(0.0),        // roughness
        ),
        "diffuse": Lambertian(
            (0.7, 0.7, 0.7),  // albedo
        ),
        "light": Lambertian(
            (2.0, 2.0, 2.0),  // albedo
        ),
    },
    objects: [
        (
            name: Some("sphere1"),
            shape: Sphere(
                (-0.5, -0.9, -1.0),  // position
                Some(1.0),           // radius
            ),
            material: "steel"
        ),
        (
            name: Some("box1"),
            shape: AxisBox(
                (1.0, -0.65, 0.7),      // position
                Some((1.25, 2.5, 1.25)) // dimensions
            ),
            material: "diffuse"
        ),
        (
            name: Some("box2"),
            shape: Box(
                (-1.8, -3.5, -1.8),     // position
                Some((3.0, 3.0, 3.0)),  // scale
                Some((0.0, 0.32, 0.0)), // rotation (in radians * PI)
            ),
            material: "diffuse"
        ),
        (
            name: Some("Tri"),
            shape: Triangle(
                (
                    (0.0, 0.0, 0.0),   // vertex A
                    (-1.0, 0.0, -1.0), // vertex B
                    (0.0, 1.0, -1.0),  // vertex C
                )
            ),
            material: "glossy",
        ),
        (
            name: Some("teapot"),
            shape: Mesh(
                ".\\scene\\models\\newell_teaset\\teapot.obj", // file path
                Some((-1.7, 1.0, 1.7)),  // translation
                Some((0.5, 0.5, 0.5)),   // scale
                Some((-0.35, 0.5, 0.0)), // rotation
                false,                   // cull backface
            ),
            material: "glass",
        ),
    ],
)

```

## Example renders

### Raytracing in One Weekend

![1679158317838](https://user-images.githubusercontent.com/47290124/227623050-1bae6c3a-85c3-4962-b701-79e064dd14a8.png)

### Cornell box

![1679114245446](https://user-images.githubusercontent.com/47290124/227622756-d5ae50ed-f101-46d4-b818-03cddfcd53ce.png)

### Utah teaset

![Teaset4](https://user-images.githubusercontent.com/47290124/228722553-80786f99-1114-4343-9dca-36e2d08ac46e.png)




