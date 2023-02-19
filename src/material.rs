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
    (r0 + (1.0 - r0) * (1.0 - cosine).powf(5.0)).clamp(0.0, 1.0)
}

impl Dielectric {
    pub fn scatter(self, ray: Ray, hit: HitRecord) -> Scatter {
        let (outward_normal, ni_over_nt, cosine, color) = if ray.dir.dot(hit.normal) > 0.0 {
            let absorbance = self.albedo * -hit.t;
            (
                -hit.normal,
                self.refractive_index,
                self.refractive_index * ray.dir.dot(hit.normal) / ray.dir.mag(),
                Vec3::new(
                    f32::exp(absorbance.x),
                    f32::exp(absorbance.y),
                    f32::exp(absorbance.z),
                ),
            )
        } else {
            (
                hit.normal,
                1.0 / self.refractive_index,
                -ray.dir.dot(hit.normal) / ray.dir.mag(),
                Vec3::one() * 0.9,
            )
        };
        let refracted = ray.dir.refracted(outward_normal, ni_over_nt);
        if refracted.mag_sq() != 0.0 {
            let reflection_prob = schlick(cosine, self.refractive_index);
            let out_dir = if fastrand::f32() < reflection_prob {
                ray.dir.reflected(outward_normal)
            } else {
                refracted
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
    pub fn lambertian(albedo: Vec3) -> Material {
        Material::Lambertian(Lambertian { albedo })
    }

    pub fn metal(albedo: Vec3, fuzz: f32, reflectance: f32) -> Material {
        Material::Metal(Metal {
            albedo,
            fuzz,
            reflectance,
        })
    }

    pub fn dielectric(albedo: Vec3, refractive_index: f32) -> Material {
        Material::Dielectric(Dielectric {
            albedo,
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
