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
    while rand_disk.mag() > 1.0 {
        rand_disk = Vec3::new(
            (fastrand::f32() * 1.9) - 1.0,
            (fastrand::f32() * 1.9) - 1.0,
            0.0,
        );
    }
    rand_disk
}

/*
#[inline]
pub fn random_unit_vector() -> Vec3 {
    random_in_unit_sphere().normalized()
}

pub fn random_in_hemisphere(normal: Vec3) -> Vec3 {
    let in_unit_sphere = random_in_unit_sphere();
    if in_unit_sphere.dot(normal) > 0.0 {
        in_unit_sphere
    } else {
        -in_unit_sphere
    }
}
*/
