const FLT_MAX: f32 = 3.40282346638528859812e+38;
const GOLDEN_RATIO: f32 = (sqrt(5.0) + 1.0) / 2.0;
const PLASTIC_NUMBER: f32 = 1.3247179572447460259609088563;
const PI: f32 = 3.1415926535897932384626433832795;
const TWO_PI: f32 = 6.2831853;
const MAX_DEPTH: u32 = 6u;
const EPSILON = 1e-3;
const OBJECT_COUNT: u32 = 4;
alias Scene = array<Sphere, OBJECT_COUNT>;
alias Materials = array<Material, OBJECT_COUNT>;

var<private> materials: Materials = Materials(Material(vec3(0.7, 0.5, 0.5), 1.), Material(vec3(0.5, 0.5, 0.9), 0.), Material(vec3(0.7, 0.9, 0.2), 0.), Material(vec3(1.), - (1.5)),);

var<private> scene: Scene = Scene(Sphere(vec3(- 1.1, 0.5, 0.), 0.5, 0), Sphere(vec3(0., 0.5, 0.), 0.5, 3), Sphere(vec3(1.1, 0.5, 0.), 0.5, 1), Sphere(vec3(0., - 2e2 - EPSILON, 0.), 2e2, 2),);
@group(0) @binding(1)
var radiance_samples_old: texture_2d<f32>;
@group(0) @binding(2)
var radiance_samples_new: texture_storage_2d<rgba32float, write>;

alias TriangleVertices = array<vec2f, 6>;
var<private> vertices: TriangleVertices = TriangleVertices(vec2f(- 1.0, 1.0), vec2f(- 1.0, - 1.0), vec2f(1.0, 1.0), vec2f(1.0, 1.0), vec2f(- 1.0, - 1.0), vec2f(1.0, - 1.0),);

@vertex
fn display_vs(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4f {
    return vec4f(vertices[vid], 0.0, 1.0);
}

struct Uniforms {
    camera: CameraUniforms,
    width: u32,
    height: u32,
    frame_num: u32,
}

struct CameraUniforms {
    origin: vec3f,
    u: vec3f,
    v: vec3f,
    w: vec3f,
}

struct Rng {
    state: u32,
}

struct Qrng {
    state: vec2f,
}

var<private> rng: Rng;
var<private> qrng: Qrng;

fn init_rng(pixel: vec2u) {
    // Seed the PRNG using the scalar index of the pixel and the current frame count.
    let seed = (pixel.x + pixel.y * uniforms.width) ^ pcg(uniforms.frame_num);
    rng.state = pcg(seed);
}

// https://www.pcg-random.org/
fn pcg(n: u32) -> u32 {
    var h = n * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    return (h >> 22u) ^ h;
}

fn pcg2d(p: vec2u) -> vec2u {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v ^= v >> vec2u(16u);
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v ^= v >> vec2u(16u);
    return v;
}

// http://www.jcgt.org/published/0009/03/02/
fn pcg3d(p: vec3u) -> vec3u {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3u(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return v;
}

fn next_pcg() -> u32 {
    rng.state = pcg(rng.state);
    return rng.state;
}

// Returns a random float in the range [0...1]. This sets the floating point exponent to zero and
// sets the most significant 23 bits of a random 32-bit unsigned integer as the mantissa. That
// generates a number in the range [1, 1.9999999], which is then mapped to [0, 0.9999999] by
// subtraction. See Ray Tracing Gems II, Section 14.3.4.
fn rand_f32() -> f32 {
    return bitcast<f32>(0x3f800000u | (next_pcg() >> 9u)) - 1.;
}

fn gen1_qrng(seed: u32) -> f32 {
    let n = f32(seed) * 0.5;
    let a1 = 1.0 / GOLDEN_RATIO;
    return (0.5 + a1 * n) % 1.0;
}

fn gen2_qrng(seed: u32) -> vec2f {
    let n = f32(seed) * 0.5;
    let a1 = 1.0 / PLASTIC_NUMBER;
    let a2 = 1.0 / (PLASTIC_NUMBER * PLASTIC_NUMBER);
    let x = (0.5 + a1 * n) % 1.0;
    let y = (0.5 + a2 * n) % 1.0;
    return vec2f(x, y);
}

fn gen_qrng_disk(seed: u32) -> vec2f {
    let rand = gen2_qrng(seed);
    let a = (2.0 * rand.x) - 1.0;
    let b = (2.0 * rand.y) - 1.0;
    let comp = (a * a) > (b * b);
    let radius = select(a, b, comp);
    let phi = select((PI / 4.0) * (b / a), (PI / 2.0) - ((PI / 4.0) * (a / b)), comp);

    return vec2f(cos(phi) * radius, sin(phi) * radius);
}

struct Sphere {
    center: vec3f,
    radius: f32,
    material: u32,
}

fn intersect_sphere(ray: Ray, sphere: Sphere) -> Hit {
    let v = ray.pos - sphere.center;
    let a = dot(ray.dir, ray.dir);
    let b = dot(v, ray.dir);
    let c = dot(v, v) - sphere.radius * sphere.radius;

    let d = b * b - a * c;
    if d < 0.0 {
        return no_hit();
    }
    let sqrt_d = sqrt(d);
    let recip_a = 1.0 / a;
    let mb = - b;

    let t1 = (mb - sqrt_d) * recip_a;
    let t2 = (mb + sqrt_d) * recip_a;

    let t = select(t2, t1, t1 > EPSILON);
    if t <= EPSILON {
        return no_hit();
    }

    let p = point_on_ray(ray, t);
    let N = ((p - sphere.center) / sphere.radius);
    return Hit(N, t, sphere.material);
}

// Uniformly sample a unit sphere centered at the origin
fn sample_sphere() -> vec3f {
    let r0 = rand_f32();
    let r1 = rand_f32();

    // Map r0 to [-1, 1]
    let y = 1. - 2. * r0;

    // Compute the projected radius on the xz-plane using Pythagorean theorem
    let xz_r = sqrt(1. - y * y);

    let phi = TWO_PI * r1;
    return vec3(xz_r * cos(phi), y, xz_r * sin(phi));
}

fn intersect_scene(ray: Ray) -> Hit {
    var closest_hit = no_hit();
    closest_hit.t = FLT_MAX;
    for (var i = 0u; i < OBJECT_COUNT; i += 1u) {
        let sphere = scene[i];
        let hit = intersect_sphere(ray, sphere);
        if hit.t > EPSILON && hit.t < closest_hit.t {
            closest_hit = hit;
        }
    }
    if closest_hit.t < FLT_MAX {
        return closest_hit;
    }
    return no_hit();
}

struct Scatter {
    attenuation: vec3f,
    ray: Ray,
}

fn sample_lambertian(normal: vec3f) -> vec3f {
    return normal + sample_sphere() * (1. - EPSILON);
}

fn schlick(cosine: f32, ior: f32) -> f32 {
    // Use Schlick's approximation for reflectance.
    let pow = (1 - cosine);
    var r0 = (1 - ior) / (1 + ior);
    r0 *= r0;
    return clamp(r0 + (1 - r0) * (pow * pow * pow * pow * pow * pow), 0.0, 1.0);
}

fn scatter(input_ray: Ray, hit: Hit, material: Material) -> Scatter {
    let incident = normalize(input_ray.dir);
    let incident_dot_normal = dot(incident, hit.normal);
    let is_front_face = incident_dot_normal < 0.;
    let N = select(- hit.normal, hit.normal, is_front_face);
    let cos_theta = abs(incident_dot_normal);

    // `ior`, `ref_ratio`, and `cannot_refract` only have meaning if the material is transmissive.
    let is_transmissive = material.specular_or_ior < 0.;
    let is_specular = material.specular_or_ior > 0.;
    let ior = abs(material.specular_or_ior);
    let ref_ratio = select(ior, 1. / ior, is_front_face);
    let cannot_refract = ref_ratio * ref_ratio * (1.0 - cos_theta * cos_theta) > 1.;

    var scattered: vec3f;
    let attenuation = material.color * 0.9;
    if is_specular || (is_transmissive && cannot_refract) || (is_transmissive && schlick(cos_theta, ior) > gen1_qrng(uniforms.frame_num)) {
        scattered = reflect(incident, N);
    }
    else if is_transmissive {
        scattered = refract(incident, N, ref_ratio);
    }
    else {
        scattered = sample_lambertian(N);
    }
    let output_ray = Ray(point_on_ray(input_ray, hit.t), normalize(scattered));
    return Scatter(attenuation, output_ray);
}

struct Ray {
    pos: vec3f,
    dir: vec3f,
}

fn point_on_ray(ray: Ray, t: f32) -> vec3<f32> {
    return ray.pos + t * ray.dir;
}

struct Hit {
    normal: vec3f,
    t: f32,
    material: u32,
}

fn no_hit() -> Hit {
    // Return invalid hit
    return Hit(normalize(vec3(0.)), - 1., 0);
}

fn is_hit_valid(hit: Hit) -> bool {
    return hit.t > EPSILON;
}

struct Material {
    color: vec3f,
    specular_or_ior: f32,
}

fn sky_color(ray: Ray) -> vec3f {
    let t = 0.5 * (normalize(ray.dir).y + 1.0);
    return (1.0 - t) * vec3(1.0) + t * vec3(0.3, 0.5, 1.0);
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn display_fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let pixels = vec2u(pos.xy);
    init_rng(pixels);
    let qrng_offset = pcg(pixels.x + pixels.y * uniforms.width) % (uniforms.width * uniforms.height);
    let blur = gen_qrng_disk(uniforms.frame_num + qrng_offset);
    let origin = uniforms.camera.origin;
    let focus_dist = 1.0;
    let aspect_ratio = f32(uniforms.width) / f32(uniforms.height);

    // Offset and normalize the viewport coordinates of the ray.
    var uv = (pos.xy + (gen2_qrng(uniforms.frame_num) - 0.5)) / vec2f(f32(uniforms.width - 1u), f32(uniforms.height - 1u));

    // Map `uv` from y-down (normalized) viewport coordinates to camera coordinates.
    uv = (2.0 * uv - vec2(1.0)) * vec2(aspect_ratio, - 1.0);

    // Compute the scene-space ray direction by rotating the camera-space vector into a new
    // basis.
    let camera_rotation = mat3x3(uniforms.camera.u, uniforms.camera.v, uniforms.camera.w);
    let direction = camera_rotation * vec3(uv, focus_dist);

    var ray = Ray(origin, direction);
    var throughput = vec3f(1.);
    var radiance_sample = vec3(0.);

    var path_length = 0u;
    while path_length < MAX_DEPTH {
        let hit = intersect_scene(ray);
        if !is_hit_valid(hit) {
            // If no intersection was found, return the color of the sky and terminate the path.
            radiance_sample += throughput * sky_color(ray);
            break;
        }

        let material = materials[hit.material];
        let scattered = scatter(ray, hit, material);
        throughput *= scattered.attenuation;
        ray = scattered.ray;
        path_length += 1u;
    }

    // Fetch the old sum of samples.
    var old_sum: vec3f;
    if uniforms.frame_num > 1 {
        old_sum = textureLoad(radiance_samples_old, vec2u(pos.xy), 0).xyz;
    }
    else {
        old_sum = vec3(0.);
    }

    // Compute and store the new sum.
    let new_sum = radiance_sample + old_sum;
    textureStore(radiance_samples_new, vec2u(pos.xy), vec4(new_sum, 0.));

    // Display the average after gamma correction (gamma = 2.2)
    let color = new_sum / f32(uniforms.frame_num);
    return vec4(pow(color, vec3(1. / 2.2)), 1.);
}