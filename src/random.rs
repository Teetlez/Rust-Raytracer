use std::f32::consts::PI;

use ultraviolet::Vec3;

pub fn random_vec() -> Vec3 {
    Vec3 {
        x: (fastrand::f32() * 1.9) - 1.0,
        y: (fastrand::f32() * 1.9) - 1.0,
        z: (fastrand::f32() * 1.9) - 1.0,
    }
}

pub fn random_in_unit_sphere() -> Vec3 {
    let mut rand_v = random_vec();
    while rand_v.mag() > 1.0 {
        rand_v = random_vec();
    }
    rand_v
}
pub fn random_in_unit_disk() -> Vec3 {
    let mut rand_disk = Vec3::new(
        (fastrand::f32() * 1.9) - 1.0,
        (fastrand::f32() * 1.9) - 1.0,
        0.0,
    );
    while rand_disk.mag_sq() > 1.0 {
        rand_disk = Vec3::new(
            (fastrand::f32() * 1.9) - 1.0,
            (fastrand::f32() * 1.9) - 1.0,
            0.0,
        );
    }
    rand_disk
}

#[inline]
pub fn random_in_cosine_sphere(r1: f32, r2: f32) -> Vec3 {
    let z = (1.0 - r2).sqrt();
    let phi = 2.0 * PI * r1;
    let x = phi.cos() * r2.sqrt();
    let y = phi.sin() * r2.sqrt();

    Vec3::new(x, y, z)
}
#[inline]
pub fn random_in_hemisphere(normal: Vec3, r1: f32, r2: f32) -> Vec3 {
    let in_unit_sphere = Onb::from_w(&normal).local(random_in_cosine_sphere(r1, r2));
    if in_unit_sphere.dot(normal) > 0.0 {
        in_unit_sphere
    } else {
        -in_unit_sphere
    }
}

/*
#[inline]
pub fn random_unit_vector() -> Vec3 {
    random_in_unit_sphere().normalized()
}

*/

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
