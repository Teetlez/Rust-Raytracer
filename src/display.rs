use ultraviolet::{Mat3, Vec3};

const M1: Mat3 = Mat3::new(
    Vec3::new(0.59719, 0.07600, 0.02840),
    Vec3::new(0.35458, 0.90834, 0.13383),
    Vec3::new(0.04823, 0.01566, 0.83777),
);
const M2: Mat3 = Mat3::new(
    Vec3::new(1.60475, -0.10208, -0.00327),
    Vec3::new(-0.53108, 1.10813, -0.07276),
    Vec3::new(-0.07367, -0.00605, 1.07602),
);

#[inline]
pub fn to_rgb(color: &Vec3, gamma: f32) -> u32 {
    let value = aces_tonemap(color, gamma);
    255 << 24
        | ((value.x * 255.4) as u32) << 16
        | ((value.y * 255.4) as u32) << 8
        | ((value.z * 255.4) as u32)
}

#[inline]
fn aces_tonemap(color: &Vec3, gamma: f32) -> Vec3 {
    let value = M1 * *color;
    let numerator = value * (value + Vec3::one() * 0.0245786) - Vec3::one() * 0.000090537;
    let denominator = value * (0.983729 * value + Vec3::one() * 0.432951) + Vec3::one() * 0.238081;
    (M2 * (numerator / denominator))
        .clamped(Vec3::zero(), Vec3::one())
        .map(|channel| channel.powf(gamma))
}
