const FLT_MAX: f32 = 3.4028234663852886e+38;
const PI: f32 = 3.141592653589793;
const TWO_PI: f32 = 6.283185307179586;
const EPSILON: f32 = 1e-4;
const SURVIVAL_BIAS: f32 = 0.01;
const AIR_INDEX: f32 = 1.00028;
const INVALID_INDEX: u32 = 0xffffffffu;
const MIN_CONVERGENCE_SAMPLES: u32 = 32u;
const CONVERGENCE_ABSOLUTE_ERROR: f32 = 1e-6;
const CONVERGENCE_RELATIVE_ERROR: f32 = 1e-3;

struct CameraUniforms {
    eye: vec3f,
    _pad0: u32,
    uvw: array<vec4f, 3>,
    hvc: array<vec4f, 3>,
    lens_rd: vec2f,
    _pad1: vec2f,
}

struct Uniforms {
    camera: CameraUniforms,
    width: u32,
    height: u32,
    frame_num: u32,
    max_bounces: u32,
    samples_per_pass: u32,
    render_mode: u32,
    hdr_width: u32,
    hdr_height: u32,
    hdr_enabled: u32,
    filter_enabled: u32,
    gamma: f32,
    light_clamp: f32,
    filter_valid: u32,
    reset_generation: u32,
    _pad3: u32,
    _pad4: u32,
}

struct BvhNode {
    min: vec4f,
    max: vec4f,
    links: vec4u,
}

struct Primitive {
    data: vec4u,
    p0: vec4f,
    p1: vec4f,
    p2: vec4f,
    n0: vec4f,
    n1: vec4f,
    n2: vec4f,
}

struct Material {
    color: vec4f,
    params: vec4f,
    info: vec4u,
}

struct Ray {
    origin: vec3f,
    direction: vec3f,
}

struct Hit {
    t: f32,
    normal: vec3f,
    material_index: u32,
    primitive_index: u32,
}

struct Scatter {
    attenuation: vec3f,
    ray: Ray,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
@group(0) @binding(1)
var<storage, read_write> sample_sums: array<vec4f>;
@group(0) @binding(2)
var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(3)
var<storage, read> primitives: array<Primitive>;
@group(0) @binding(4)
var<storage, read> materials: array<Material>;
@group(0) @binding(5)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(6)
var<storage, read_write> filter_a: array<vec4f>;
@group(0) @binding(7)
var<storage, read_write> filter_b: array<vec4f>;
@group(0) @binding(8)
var<storage, read_write> pixel_stats: array<vec4u>;

var<private> rng_state: u32;

fn pcg(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    state = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (state >> 22u) ^ state;
}

fn random_f32() -> f32 {
    rng_state = pcg(rng_state);
    return f32(rng_state >> 8u) * (1.0 / 16777216.0);
}

fn pow5(value: f32) -> f32 {
    let square = value * value;
    return square * square * value;
}

fn random_unit_vector() -> vec3f {
    let y = 1.0 - 2.0 * random_f32();
    let radius = sqrt(max(0.0, 1.0 - y * y));
    let angle = TWO_PI * random_f32();
    return vec3f(radius * cos(angle), y, radius * sin(angle));
}

fn random_in_unit_sphere() -> vec3f {
    return random_unit_vector() * pow(random_f32(), 1.0 / 3.0);
}

fn sample_hemisphere(normal: vec3f) -> vec3f {
    let direction = random_unit_vector();
    return select(- direction, direction, dot(direction, normal) >= 0.0);
}

fn no_hit() -> Hit {
    return Hit(FLT_MAX, vec3f(0.0, 1.0, 0.0), 0u, INVALID_INDEX);
}

fn intersect_sphere(ray: Ray, primitive: Primitive, primitive_index: u32, max_t: f32) -> Hit {
    let offset = ray.origin - primitive.p0.xyz;
    let half_b = dot(offset, ray.direction);
    let discriminant = half_b * half_b - dot(offset, offset) + primitive.p0.w * primitive.p0.w;
    if discriminant <= 0.0 {
        return no_hit();
    }
    let root = sqrt(discriminant);
    var t = - half_b - root;
    if t <= EPSILON || t >= max_t {
        t = - half_b + root;
    }
    if t <= EPSILON || t >= max_t {
        return no_hit();
    }
    return Hit(t, normalize(ray.origin + t * ray.direction - primitive.p0.xyz), primitive.data.y, primitive_index,);
}

fn intersect_triangle(ray: Ray, primitive: Primitive, primitive_index: u32, max_t: f32) -> Hit {
    let edge1 = primitive.p1.xyz - primitive.p0.xyz;
    let edge2 = primitive.p2.xyz - primitive.p0.xyz;
    let h = cross(ray.direction, edge2);
    let determinant = dot(edge1, h);
    if (primitive.data.z == 0u && determinant < 0.0) || abs(determinant) < 1e-6 {
        return no_hit();
    }
    let inverse_determinant = 1.0 / determinant;
    let offset = ray.origin - primitive.p0.xyz;
    let u = inverse_determinant * dot(offset, h);
    if u < 0.0 || u > 1.0 {
        return no_hit();
    }
    let q = cross(offset, edge1);
    let v = inverse_determinant * dot(ray.direction, q);
    if v < 0.0 || u + v > 1.0 {
        return no_hit();
    }
    let t = inverse_determinant * dot(edge2, q);
    if t <= EPSILON || t >= max_t {
        return no_hit();
    }
    let normal = normalize((1.0 - u - v) * primitive.n0.xyz + u * primitive.n1.xyz + v * primitive.n2.xyz,);
    return Hit(t, normal, primitive.data.y, primitive_index);
}

fn intersect_primitive(ray: Ray, index: u32, max_t: f32) -> Hit {
    let primitive = primitives[index];
    if primitive.data.x == 0u {
        return intersect_sphere(ray, primitive, index, max_t);
    }
    return intersect_triangle(ray, primitive, index, max_t);
}

fn hit_bounds(ray: Ray, node: BvhNode, max_t: f32) -> bool {
    var near_t = EPSILON;
    var far_t = max_t;
    for (var axis = 0u; axis < 3u; axis += 1u) {
        if abs(ray.direction[axis]) < 1e-10 {
            if ray.origin[axis] < node.min[axis] || ray.origin[axis] > node.max[axis] {
                return false;
            }
        }
        else {
            let inverse = 1.0 / ray.direction[axis];
            let first = (node.min[axis] - ray.origin[axis]) * inverse;
            let second = (node.max[axis] - ray.origin[axis]) * inverse;
            near_t = max(near_t, min(first, second));
            far_t = min(far_t, max(first, second));
            if far_t < near_t {
                return false;
            }
        }
    }
    return true;
}

fn intersect_scene(ray: Ray) -> Hit {
    var closest = no_hit();
    var node_index = 0u;
    let node_count = arrayLength(&bvh_nodes);
    while node_index < node_count {
        let node = bvh_nodes[node_index];
        if !hit_bounds(ray, node, closest.t) {
            if node.links.x == INVALID_INDEX {
                node_index = node.links.y;
            }
            else {
                node_index = node.links.z;
            }
            continue;
        }
        if node.links.x == INVALID_INDEX {
            for (var offset = 0u; offset < node.links.w; offset += 1u) {
                let hit = intersect_primitive(ray, node.links.z + offset, closest.t);
                if hit.primitive_index != INVALID_INDEX {
                    closest = hit;
                }
            }
            node_index = node.links.y;
        }
        else {
            node_index = node.links.x;
        }
    }
    return closest;
}

fn schlick(cosine: f32, from_ior: f32, to_ior: f32) -> f32 {
    let r0 = pow((from_ior - to_ior) / (from_ior + to_ior), 2.0);
    return clamp(r0 + (1.0 - r0) * pow5(1.0 - cosine), 0.0, 1.0);
}

fn scatter(ray: Ray, hit: Hit, material: Material) -> Scatter {
    let kind = material.info.x;
    let color = material.color.xyz;
    let roughness = material.params.x;
    let incident = normalize(ray.direction);
    var normal = hit.normal;
    var attenuation = color;
    var direction = vec3f(0.0);

    if kind == 0u {
        direction = sample_hemisphere(normal);
    }
    else if kind == 1u {
        normal = normalize(normal + random_in_unit_sphere() * roughness);
        let cosine = dot(- incident, normal);
        let reflection_probability = schlick(cosine, AIR_INDEX, 1.0 + material.params.y);
        if random_f32() <= reflection_probability {
            direction = reflect(incident, normal);
            attenuation = vec3f(0.9);
        }
        else {
            direction = sample_hemisphere(hit.normal);
        }
    }
    else if kind == 2u {
        normal = normalize(normal + random_in_unit_sphere() * roughness);
        direction = reflect(incident, normal);
        let cosine = dot(- incident, normal);
        attenuation = clamp(color + (vec3f(1.0) - color) * pow5(1.0 - cosine), vec3f(0.0), vec3f(1.0),);
    }
    else {
        normal = normalize(normal + random_in_unit_sphere() * roughness);
        let entering = dot(incident, hit.normal) <= 0.0;
        let outward_normal = select(- normal, normal, entering);
        let from_ior = select(material.params.z, AIR_INDEX, entering);
        let to_ior = select(AIR_INDEX, material.params.z, entering);
        let ratio = from_ior / to_ior;
        let cosine = clamp(abs(dot(incident, normal)), 0.0, 1.0);
        let cannot_refract = ratio * ratio * (1.0 - cosine * cosine) > 1.0;
        if cannot_refract || random_f32() < schlick(cosine, from_ior, to_ior) {
            direction = reflect(incident, outward_normal);
        }
        else {
            direction = refract(incident, outward_normal, ratio);
        }
        attenuation = select(exp(- color * hit.t * 2.0), vec3f(0.9), entering);
    }
    return Scatter(attenuation, Ray(ray.origin + hit.t * ray.direction, normalize(direction)));
}

fn sky_color(ray: Ray) -> vec3f {
    let direction = normalize(ray.direction);
    if uniforms.hdr_enabled != 0u {
        let u = 0.5 + atan2(direction.x, direction.z) / TWO_PI;
        let v = 0.5 + asin(clamp(direction.y, - 1.0, 1.0)) / PI;
        let x = i32(clamp(u, 0.0, 0.999999) * f32(uniforms.hdr_width - 1u));
        let y = i32((1.0 - clamp(v, 0.0, 1.0)) * f32(uniforms.hdr_height - 1u));
        let hdr = textureLoad(hdr_texture, vec2i(x, y), 0).xyz;
        return clamp(hdr, vec3f(0.0), vec3f(uniforms.light_clamp));
    }
    let sun = normalize(vec3f(- 1.0, 0.75, 0.5));
    let t = 0.5 * (dot(direction, sun) + 1.0);
    let bottom = vec3f(1.0);
    let top = vec3f(0.1, 0.3, 0.8);
    return (bottom + (top - bottom) * t) * 2.0;
}

fn trace_ray(ray: Ray) -> vec3f {
    if uniforms.render_mode != 0u {
        let hit = intersect_scene(ray);
        if hit.primitive_index == INVALID_INDEX {
            return sky_color(ray);
        }
        if uniforms.render_mode == 1u {
            return (hit.normal + vec3f(1.0)) * 0.5;
        }
        let depth_shading = clamp(1.0 - hit.t * 0.01, 0.35, 1.0);
        return materials[hit.material_index].color.xyz * depth_shading;
    }

    var current_ray = ray;
    var throughput = vec3f(1.0);
    for (var bounce = 0u; bounce < uniforms.max_bounces; bounce += 1u) {
        let hit = intersect_scene(current_ray);
        if hit.primitive_index == INVALID_INDEX {
            return throughput * sky_color(current_ray);
        }
        let scattered = scatter(current_ray, hit, materials[hit.material_index]);
        let max_attenuation = max(scattered.attenuation.x, max(scattered.attenuation.y, scattered.attenuation.z));
        if max_attenuation <= 1.0 {
            throughput *= scattered.attenuation;
            if random_f32() > max_attenuation {
                return throughput * SURVIVAL_BIAS;
            }
            throughput /= max(max_attenuation, 1e-8);
            current_ray = scattered.ray;
        }
        else {
            let emission = clamp(scattered.attenuation, vec3f(0.0), vec3f(uniforms.light_clamp));
            return throughput * emission;
        }
    }
    return throughput * SURVIVAL_BIAS;
}

fn camera_ray(pixel: vec2u) -> Ray {
    let jitter_x = random_f32();
    let jitter_y = random_f32();
    let lens_radius = sqrt(random_f32()) * uniforms.camera.lens_rd.x;
    let lens_angle = TWO_PI * random_f32();
    let lens_offset = cos(lens_angle) * lens_radius * uniforms.camera.uvw[0].xyz + sin(lens_angle) * lens_radius * uniforms.camera.uvw[1].xyz;
    let denominator_x = f32(max(uniforms.width - 1u, 1u));
    let denominator_y = f32(max(uniforms.height - 1u, 1u));
    let s = (f32(pixel.x) + jitter_y) / denominator_x;
    let t = (f32(uniforms.height - 1u - pixel.y) + jitter_x) / denominator_y;
    let viewport_point = uniforms.camera.hvc[2].xyz + s * uniforms.camera.hvc[0].xyz + t * uniforms.camera.hvc[1].xyz;
    return Ray(uniforms.camera.eye + lens_offset, normalize(viewport_point - uniforms.camera.eye - lens_offset),);
}

@compute @workgroup_size(8, 8, 1)
fn trace_compute(@builtin(global_invocation_id) invocation: vec3u) {
    if invocation.x >= uniforms.width || invocation.y >= uniforms.height {
        return;
    }
    let pixel_index = invocation.x + invocation.y * uniforms.width;
    let previous_stats = pixel_stats[pixel_index];
    let has_previous_stats = previous_stats.z == uniforms.reset_generation;
    let previous_count = select(0u, previous_stats.y, has_previous_stats);
    let previous_m2 = select(0.0, bitcast<f32>(previous_stats.x), has_previous_stats);
    let previous_mean_color = select(vec3f(0.0), sample_sums[pixel_index].xyz, has_previous_stats,);
    if previous_count >= MIN_CONVERGENCE_SAMPLES {
        let count = f32(previous_count);
        let variance = max(previous_m2 / (count - 1.0), 0.0);
        let standard_error = sqrt(variance / count);
        let tolerance = max(CONVERGENCE_ABSOLUTE_ERROR, length(previous_mean_color) * CONVERGENCE_RELATIVE_ERROR,);
        if standard_error <= tolerance {
            return;
        }
    }

    rng_state = pcg(pixel_index ^ pcg(uniforms.frame_num * 747796405u + 2891336453u));
    var pass_sum = vec3f(0.0);
    var pass_mean = vec3f(0.0);
    var pass_m2 = vec3f(0.0);
    var pass_count = 0u;
    for (var sample_index = 0u; sample_index < uniforms.samples_per_pass; sample_index += 1u) {
        let sample_color = trace_ray(camera_ray(invocation.xy));
        pass_sum += sample_color;
        pass_count += 1u;
        let delta = sample_color - pass_mean;
        pass_mean += delta / f32(pass_count);
        pass_m2 += delta * (sample_color - pass_mean);
    }
    let total_count = previous_count + pass_count;
    let color_delta = pass_mean - previous_mean_color;
    let mean_color = previous_mean_color + color_delta * (f32(pass_count) / f32(total_count));
    let count_before = f32(previous_count);
    let batch_count = f32(pass_count);
    let combined_m2 = previous_m2 + dot(pass_m2, vec3f(1.0)) + dot(color_delta, color_delta) * count_before * batch_count / f32(total_count);
    sample_sums[pixel_index] = vec4f(mean_color, 1.0);
    pixel_stats[pixel_index] = vec4u(bitcast<u32>(combined_m2), total_count, uniforms.reset_generation, 0u,);
}

fn filter_source(index: u32, source_kind: u32) -> vec3f {
    if source_kind == 0u {
        return sample_sums[index].xyz;
    }
    if source_kind == 1u {
        return filter_a[index].xyz;
    }
    return filter_b[index].xyz;
}

fn bilateral_filter_pixel(pixel: vec2u, source_kind: u32, destination_kind: u32, diameter: u32, sigma_i: f32, sigma_s: f32,) {
    let index = pixel.x + pixel.y * uniforms.width;
    let center = filter_source(index, source_kind);
    let center_intensity = (center.x + center.y + center.z) / 3.0;
    let half = i32(diameter / 2u);
    var filtered = vec3f(0.0);
    var total_weight = 0.0;
    for (var offset_y = 0; offset_y < i32(diameter); offset_y += 1) {
        for (var offset_x = 0; offset_x < i32(diameter); offset_x += 1) {
            let x = clamp(i32(pixel.x) + offset_x - half, 0, i32(uniforms.width) - 1);
            let y = clamp(i32(pixel.y) + offset_y - half, 0, i32(uniforms.height) - 1);
            let neighbor_position = vec2u(u32(x), u32(y));
            let neighbor_index = neighbor_position.x + neighbor_position.y * uniforms.width;
            let neighbor = filter_source(neighbor_index, source_kind);
            let neighbor_intensity = (neighbor.x + neighbor.y + neighbor.z) / 3.0;
            let intensity_delta = neighbor_intensity - center_intensity;
            let spatial_delta = vec2f(f32(x) - f32(pixel.x), f32(y) - f32(pixel.y));
            let intensity_weight = exp(- (intensity_delta * intensity_delta) / (2.0 * sigma_i * sigma_i));
            let spatial_weight = exp(- dot(spatial_delta, spatial_delta) / (2.0 * sigma_s * sigma_s));
            let weight = intensity_weight * spatial_weight;
            filtered += neighbor * weight;
            total_weight += weight;
        }
    }
    let result = vec4f(filtered / max(total_weight, 1e-20), 1.0);
    if destination_kind == 1u {
        filter_a[index] = result;
    }
    else {
        filter_b[index] = result;
    }
}

@compute @workgroup_size(8, 8, 1)
fn filter_pass_1(@builtin(global_invocation_id) invocation: vec3u) {
    if uniforms.filter_enabled == 0u || invocation.x >= uniforms.width || invocation.y >= uniforms.height {
        return;
    }
    bilateral_filter_pixel(invocation.xy, 0u, 1u, 9u, 0.05, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn filter_pass_2(@builtin(global_invocation_id) invocation: vec3u) {
    if uniforms.filter_enabled == 0u || invocation.x >= uniforms.width || invocation.y >= uniforms.height {
        return;
    }
    bilateral_filter_pixel(invocation.xy, 1u, 2u, 4u, 0.025, 0.5);
}

@compute @workgroup_size(8, 8, 1)
fn filter_pass_3(@builtin(global_invocation_id) invocation: vec3u) {
    if uniforms.filter_enabled == 0u || invocation.x >= uniforms.width || invocation.y >= uniforms.height {
        return;
    }
    bilateral_filter_pixel(invocation.xy, 2u, 1u, 3u, 0.05 / 3.0, 1.0 / 3.0);
}

const M1 = mat3x3f(vec3f(0.59719, 0.07600, 0.02840), vec3f(0.35458, 0.90834, 0.13383), vec3f(0.04823, 0.01566, 0.83777),);
const M2 = mat3x3f(vec3f(1.60475, - 0.10208, - 0.00327), vec3f(- 0.53108, 1.10813, - 0.07276), vec3f(- 0.07367, - 0.00605, 1.07602),);

fn aces_tonemap(color: vec3f) -> vec3f {
    let v = M1 * color;
    let a = v * (v + vec3f(0.0245786)) - vec3f(0.000090537);
    let b = v * (0.983729 * v + vec3f(0.432951)) + vec3f(0.238081);
    return pow(clamp(M2 * (a / b), vec3f(0.0), vec3f(1.0)), vec3f(uniforms.gamma));
}

@vertex
fn display_vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    var position = vec2f(- 1.0, - 1.0);
    if index == 1u {
        position = vec2f(3.0, - 1.0);
    }
    else if index == 2u {
        position = vec2f(- 1.0, 3.0);
    }
    return vec4f(position, 0.0, 1.0);
}

@fragment
fn display_fs(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let x = min(u32(position.x), uniforms.width - 1u);
    let y = min(u32(position.y), uniforms.height - 1u);
    let index = x + y * uniforms.width;
    var average = sample_sums[index].xyz;
    if uniforms.filter_enabled != 0u && uniforms.filter_valid != 0u {
        average = filter_a[index].xyz;
    }
    return vec4f(aces_tonemap(average), 1.0);
}