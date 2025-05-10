const FLT_MAX: f32 = 3.40282346638528859812e+38;
const OBJECT_COUNT: u32 = 2;
const GOLDEN_RATIO: f32 = (sqrt(5.0) + 1.0) / 2.0;
const PLASTIC_NUMBER: f32 = 1.3247179572447460259609088563;
alias Scene = array<Sphere, OBJECT_COUNT>;
var<private> scene: Scene = Scene(Sphere(vec3(0., 0., - 1.), 0.5), Sphere(vec3(0., - 100.5, - 1.), 100.),);

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
    width: u32,
    height: u32,
    frame_num: u32,
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
    let seed = (pixel.x + pixel.y * uniforms.width) ^ jenkins_hash(uniforms.frame_num);
    rng.state = jenkins_hash(seed);
}

// A slightly modified version of the "One-at-a-Time Hash" function by Bob Jenkins.
// See https://www.burtleburtle.net/bob/hash/doobs.html
fn jenkins_hash(i: u32) -> u32 {
    var x = i;
    x += x << 10u;
    x ^= x >> 6u;
    x += x << 3u;
    x ^= x >> 11u;
    x += x << 15u;
    return x;
}

// The 32-bit "xor" function from Marsaglia G., "Xorshift RNGs", Section 3.
fn xorshift32() -> u32 {
    var x = rng.state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    rng.state = x;
    return x;
}

// Returns a random float in the range [0...1]. This sets the floating point exponent to zero and
// sets the most significant 23 bits of a random 32-bit unsigned integer as the mantissa. That
// generates a number in the range [1, 1.9999999], which is then mapped to [0, 0.9999999] by
// subtraction. See Ray Tracing Gems II, Section 14.3.4.
fn rand_f32() -> f32 {
    return bitcast<f32>(0x3f800000u | (xorshift32() >> 9u)) - 1.;
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

struct Sphere {
    center: vec3f,
    radius: f32,
}

fn intersect_sphere(ray: Ray, sphere: Sphere) -> Hit {
    let v = ray.pos - sphere.center;
    let a = dot(ray.dir, ray.dir);
    let b = dot(v, ray.dir);

    let d = b * b - a * (dot(v, v) - sphere.radius * sphere.radius);
    if d < 0.0 {
        return no_hit();
    }
    let sqrt_d = sqrt(d);
    let recip_a = 1.0 / a;
    let mb = - b;

    let t1 = (mb - sqrt_d) * recip_a;
    let t2 = (mb + sqrt_d) * recip_a;

    let t = select(t2, t1, t1 > 0.);
    if t <= 0. {
        return no_hit();
    }

    let p = point_on_ray(ray, t);
    let N = (p - sphere.center) / sphere.radius;
    return Hit(N, t);
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
}

fn no_hit() -> Hit {
    // Return invalid hit
    return Hit(vec3(0.0), - 1.0);
}

fn sky_color(ray: Ray) -> vec3f {
    let t = 0.5 * (normalize(ray.dir).y + 1.0);
    return (1.0 - t) * vec3(1.0) + t * vec3(0.3, 0.5, 1.0);
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn display_fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let origin = vec3(0.0);
    let focus_dist = 1.0;
    let aspect_ratio = f32(uniforms.width) / f32(uniforms.height);

    // Offset and normalize the viewport coordinates of the ray.
    var uv = (pos.xy + (gen2_qrng(uniforms.frame_num) - 0.5)) / vec2f(f32(uniforms.width - 1u), f32(uniforms.height - 1u));

    // Normalize viewport coords
    // var uv = pos.xy / vec2f(f32(uniforms.width - 1u), f32(uniforms.height - 1u));

    uv = (2.0 * uv - vec2(1.0)) * vec2(aspect_ratio, - 1.0);
    let direction = vec3(uv, - focus_dist);
    let ray = Ray(origin, direction);

    var closest_hit = Hit(vec3(0.), FLT_MAX);
    for (var i = 0u; i < OBJECT_COUNT; i += 1u) {
        let sphere = scene[i];
        let hit = intersect_sphere(ray, sphere);
        if hit.t > 0. && hit.t < closest_hit.t {
            closest_hit = hit;
        }
    }

    var radiance_sample: vec3f;
    if closest_hit.t - 0.00001 < FLT_MAX {
        radiance_sample = vec3(0.5 * closest_hit.normal + vec3(0.5));
    }
    else {
        radiance_sample = sky_color(ray);
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

    // Display the average.
    return vec4(new_sum / f32(uniforms.frame_num), 1.);
}