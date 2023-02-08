use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[inline]
    pub fn zero() -> Vec3 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn one() -> Vec3 {
        Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    #[inline]
    pub fn dot(&self, rhs: Vec3) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline]
    pub fn length_sq(&self) -> f32 {
        self.dot(*self)
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline]
    pub fn cross(&self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: -self.x * rhs.z + self.z * rhs.x,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn normalize(&self) -> Vec3 {
        let inv = 1.0 / self.length();
        Vec3 {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
        }
    }

    #[inline]
    pub fn reflect(&self, normal: Vec3) -> Vec3 {
        *self - 2.0 * self.dot(normal) * normal
    }

    #[inline]
    pub fn refract(&self, normal: Vec3, ni_over_nt: f32) -> Option<Vec3> {
        let uv = self.normalize();
        let dt = uv.dot(normal);
        let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
        if discriminant > 0.0 {
            Some(ni_over_nt * (uv - dt * normal) - discriminant.sqrt() * normal)
        } else {
            None
        }
    }

    #[inline]
    pub fn random() -> Vec3 {
        Vec3::new(fastrand::f32(), fastrand::f32(), fastrand::f32())
    }

    pub fn random_in_unit_sphere() -> Vec3 {
        let mut rand_v = (2.0 * Vec3::random()) - Vec3::one();
        while rand_v.length_sq() > 1.0 {
            rand_v = (2.0 * Vec3::random()) - Vec3::one();
        }
        rand_v
    }
    pub fn random_in_unit_disk() -> Vec3 {
        let mut rand_disk = Vec3::new(
            (fastrand::f32() * 1.9) - 1.0,
            (fastrand::f32() * 1.9) - 1.0,
            0.0,
        );
        while rand_disk.length_sq() > 1.0 {
            rand_disk = Vec3::new(
                (fastrand::f32() * 1.9) - 1.0,
                (fastrand::f32() * 1.9) - 1.0,
                0.0,
            );
        }
        rand_disk
    }

    #[inline]
    pub fn random_unit_vector() -> Vec3 {
        Vec3::random_in_unit_sphere().normalize()
    }

    pub fn random_in_hemisphere(normal: Vec3) -> Vec3 {
        let in_unit_sphere = Vec3::random_in_unit_sphere();
        if in_unit_sphere.dot(normal) > 0.0 {
            in_unit_sphere
        } else {
            -in_unit_sphere
        }
    }

    #[inline]
    pub fn near_zero(&self) -> bool {
        self.length_sq() < 1e-8
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Vec3;

    #[test]
    fn add() {
        assert_eq!(
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 2.0
            } + Vec3 {
                x: 2.0,
                y: 1.0,
                z: 2.0
            },
            Vec3 {
                x: 3.0,
                y: 1.0,
                z: 4.0
            }
        );
    }

    #[test]
    fn cross() {
        assert_eq!(
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 2.0
            }
            .cross(Vec3 {
                x: 2.0,
                y: 1.0,
                z: 2.0
            }),
            Vec3 {
                x: -2.0,
                y: 2.0,
                z: 1.0
            }
        );
    }

    #[test]
    fn dot() {
        assert_eq!(
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 2.0
            }
            .dot(Vec3 {
                x: 2.0,
                y: 1.0,
                z: 2.0
            }),
            6.0
        );
    }

    #[test]
    fn length() {
        let v = Vec3 {
            x: -2.0,
            y: -2.0,
            z: -1.0,
        };
        let u = Vec3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        assert_eq!(v.length(), 3.0);
        assert_eq!(u.length(), 1.0);
    }

    #[test]
    fn length_sq() {
        let v = Vec3 {
            x: -2.0,
            y: -2.0,
            z: -1.0,
        };
        let u = Vec3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        assert_eq!(v.length_sq(), 9.0);
        assert_eq!(u.length_sq(), 1.0);
    }

    #[test]
    fn mul() {
        assert_eq!(
            3.0 * Vec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0
            },
            Vec3 {
                x: 3.0,
                y: 6.0,
                z: 9.0
            }
        );
    }

    #[test]
    fn neg() {
        assert_eq!(
            -Vec3 {
                x: 1.0,
                y: -2.0,
                z: 3.0
            },
            Vec3 {
                x: -1.0,
                y: 2.0,
                z: -3.0
            }
        );
    }

    #[test]
    fn sub() {
        assert_eq!(
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 2.0
            } - Vec3 {
                x: 2.0,
                y: 1.0,
                z: 2.0
            },
            Vec3 {
                x: -1.0,
                y: -1.0,
                z: 0.0
            }
        );
    }
}
