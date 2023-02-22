use crate::{hittable::HitRecord, random::random_in_unit_sphere, ray::Ray};

use ultraviolet::Vec3;

#[derive(Copy, Clone)]
pub struct Scatter {
    pub attenuation: Vec3,
    pub ray: Ray,
}

impl Scatter {
    pub fn new(attenuation: Vec3, ray: Ray) -> Scatter {
        Scatter { attenuation, ray }
    }
}

#[derive(Copy, Clone)]
pub struct Lambertian {
    pub albedo: Vec3,
}

impl Lambertian {
    pub fn scatter(self, _: Ray, hit: HitRecord) -> Scatter {
        let target = hit.point + hit.normal + random_in_unit_sphere();
        let attenuation = self.albedo;
        let scattered_ray = Ray::new(hit.point, target - hit.point);
        Scatter::new(attenuation, scattered_ray)
    }
}

#[derive(Copy, Clone)]
pub struct Metal {
    pub albedo: Vec3,
    pub fuzz: f32,
    pub reflectance: f32,
}

impl Metal {
    pub fn scatter(self, ray: Ray, hit: HitRecord) -> Scatter {
        let out_dir = ray.dir.reflected(hit.normal)
            + (self.fuzz
                * (1.9
                    - schlick(
                        self.reflectance * ray.dir.dot(hit.normal) / ray.dir.mag(),
                        self.reflectance,
                    ))
                * random_in_unit_sphere());
        Scatter::new(self.albedo, Ray::new(hit.point, out_dir))
    }
}

#[derive(Copy, Clone)]
pub struct Dielectric {
    pub albedo: Vec3,
    pub refractive_index: f32,
}

fn schlick(cosine: f32, refractive_index: f32) -> f32 {
    let mut r0 = (1.0 - refractive_index) / (1.0 + refractive_index);
    r0 = r0 * r0;
    (r0 + (1.0 - r0) * (1.0 - cosine).powi(5)).clamp(0.0, 1.0)
}

impl Dielectric {
    pub fn scatter(self, ray: Ray, hit: HitRecord) -> Scatter {
        let (outward_normal, ni_over_nt, cosine, color) = if ray.dir.dot(hit.normal) > 0.0 {
            let absorbance = self.albedo * -hit.t;
            (
                -hit.normal,
                self.refractive_index / 1.00028,
                ((-ray.dir).dot(-hit.normal)).min(1.0),
                Vec3::new(
                    f32::exp(absorbance.x),
                    f32::exp(absorbance.y),
                    f32::exp(absorbance.z),
                ),
            )
        } else {
            (
                hit.normal,
                1.00028 / self.refractive_index,
                ((-ray.dir).dot(hit.normal)).min(1.0),
                Vec3::one() * 0.9,
            )
        };
        if ni_over_nt * (1.0 - (cosine * cosine)).sqrt() <= 1.0 {
            let reflection_prob = schlick(cosine, ni_over_nt);
            let out_dir = if fastrand::f32() < reflection_prob {
                ray.dir.reflected(outward_normal)
            } else {
                ray.dir.refracted(outward_normal, ni_over_nt)
            };
            Scatter::new(color, Ray::new(hit.point, out_dir))
        } else {
            Scatter::new(
                color,
                Ray::new(hit.point, ray.dir.reflected(outward_normal)),
            )
        }
    }
}

#[derive(Copy, Clone)]
pub enum Material {
    Dielectric(Dielectric),
    Lambertian(Lambertian),
    Metal(Metal),
}

impl Material {
    pub fn lambertian(albedo: (f32, f32, f32)) -> Material {
        Material::Lambertian(Lambertian {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
        })
    }

    pub fn metal(albedo: (f32, f32, f32), fuzz: f32, reflectance: f32) -> Material {
        Material::Metal(Metal {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            fuzz,
            reflectance,
        })
    }

    pub fn dielectric(albedo: (f32, f32, f32), refractive_index: f32) -> Material {
        Material::Dielectric(Dielectric {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            refractive_index,
        })
    }

    pub fn scatter(self, ray: Ray, hit: HitRecord) -> Scatter {
        match hit.material {
            Material::Dielectric(d) => d.scatter(ray, hit),
            Material::Lambertian(l) => l.scatter(ray, hit),
            Material::Metal(m) => m.scatter(ray, hit),
        }
    }
}
