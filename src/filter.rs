use core::f32::consts::PI;
use ultraviolet::Vec3;

fn distance(x: u32, y: u32, i: u32, j: u32) -> f32 {
    let dx = x - i;
    let dy = y - j;
    ((dx * dx + dy * dy) as f32).sqrt()
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    (-(x * x) / (2.0 * sigma * sigma)).exp() / (2.0 * PI * sigma * sigma)
}

pub fn bilateral_filter(
    pixel: &Vec3,
    current: usize,
    img: &[Vec3],
    dimension: (u32, u32),
    diameter: u32,
    sigma_i: f32,
    sigma_s: f32,
) -> Vec3 {
    let (x, y) = (current as u32 % dimension.0, current as u32 / dimension.0);
    let current_sum = pixel.x + pixel.y + pixel.z;
    let sum_scale = 1.0 / 3.0;

    let mut filtered = Vec3::zero();
    let mut pixel_weight: f32 = 0.0;
    let half = diameter / 2;

    (0..diameter).for_each(|i| {
        (0..diameter).for_each(|j| {
            let x_neighbor = (x - (half - i)).clamp(0, dimension.0 - 1);
            let y_neighbor = (y - (half - j)).clamp(0, dimension.1 - 1);
            let neighbor = img[((y_neighbor * dimension.0) + x_neighbor) as usize];
            let neighbor_sum = neighbor.x + neighbor.y + neighbor.z;

            let gauss_i = gaussian(sum_scale * (neighbor_sum - current_sum), sigma_i);
            let gauss_s = gaussian(distance(x, y, x_neighbor, y_neighbor), sigma_s);
            let weight = gauss_i * gauss_s;

            filtered += neighbor * weight;
            pixel_weight += weight;
        })
    });

    filtered / pixel_weight
}
