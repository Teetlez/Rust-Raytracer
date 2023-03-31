use ultraviolet::Vec3;

#[derive(Copy, Clone, Debug)]
pub struct Ray {
    pub pos: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(pos: Vec3, dir: Vec3) -> Ray {
        Ray {
            pos,
            dir: dir.normalized(),
        }
    }

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.pos + t * self.dir
    }
}

pub struct Onb {
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
