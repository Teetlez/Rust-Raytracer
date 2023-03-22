use std::f32::consts::PI;

use ultraviolet::Vec3;

#[inline]
pub fn quasirandom_in_unit_sphere(r1: f32, r2: f32) -> Vec3 {
    let rand_unit_vec = quasirandom_on_uniform_sphere(r1, r2);
    rand_unit_vec * fastrand::f32()
}

#[inline]
pub fn quasirandom_in_cocentric_disk(r1: f32, r2: f32) -> Vec3 {
    let (a, b) = ((2.0 * r1) - 1.0, (2.0 * r2) - 1.0);
    let (radius, phi) = if (a * a) > (b * b) {
        (a, (PI / 4.0) * (b / a))
    } else {
        (b, (PI / 2.0) - ((PI / 4.0) * (a / b)))
    };
    Vec3::new(phi.cos() * radius, phi.sin() * radius, 0.0)
}

#[inline]
pub fn quasirandom_on_cosine_sphere(r1: f32, r2: f32) -> Vec3 {
    let z = (1.0 - r2).sqrt();
    let phi = 2.0 * PI * r1;
    let x = phi.cos() * r2.sqrt();
    let y = phi.sin() * r2.sqrt();

    Vec3::new(x, y, z)
}

#[inline]
pub fn quasirandom_on_uniform_sphere(r1: f32, r2: f32) -> Vec3 {
    let phi = 2.0 * PI * r1;
    let theta = ((2.0 * r2) - 1.0).acos();
    let x = phi.cos() * theta.sin();
    let y = phi.sin() * theta.sin();
    let z = theta.cos();

    Vec3::new(x, y, z)
}

#[inline]
pub fn quasirandom_on_hemisphere(normal: Vec3, r1: f32, r2: f32) -> Vec3 {
    Onb::from_w(&normal).local(quasirandom_on_cosine_sphere(r1, r2))
}

#[inline]
pub fn random_in_unit_sphere() -> Vec3 {
    quasirandom_in_unit_sphere(fastrand::f32(), fastrand::f32())
}

struct Onb {
    pub u: Vec3,
    pub v: Vec3,
    pub w: Vec3,
}

impl Onb {
    pub fn _new(u: Vec3, v: Vec3, w: Vec3) -> Onb {
        Onb { u, v, w }
    }

    #[inline]
    pub fn from_w(n: &Vec3) -> Onb {
        let w = n.normalized();
        let a = if w.x.abs() > 0.9 {
            Vec3::unit_y()
        } else {
            Vec3::unit_x()
        };
        let v = w.cross(a).normalized();
        let u = w.cross(v);
        Onb { u, v, w }
    }

    #[inline]
    pub fn local(&self, a: Vec3) -> Vec3 {
        a.x * self.u + a.y * self.v + a.z * self.w
    }
}
