use crate::{
    hittable::HitRecord,
    random::{random_in_hemisphere, random_in_unit_sphere},
    ray::Ray,
};

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
    pub fn scatter(self, _: Ray, hit: HitRecord, scatter_rng: &[(f32, f32)]) -> Scatter {
        let (r1, r2) = scatter_rng[fastrand::usize(0..scatter_rng.len())];
        let direction = random_in_hemisphere(hit.normal, r1, r2);
        let attenuation = self.albedo;
        let scattered_ray = Ray::new(hit.point, direction);
        Scatter::new(attenuation, scattered_ray)
    }
}

#[derive(Copy, Clone)]
pub struct Glossy {
    pub albedo: Vec3,
    pub roughness: f32,
    pub reflectance: f32,
}

impl Glossy {
    pub fn scatter(self, ray: Ray, hit: HitRecord, scatter_rng: &[(f32, f32)]) -> Scatter {
        let reflection_prob = schlick(
            ((-ray.dir).dot(hit.normal)).min(1.0),
            1.00028 / (1.0 + self.reflectance),
        );
        let (r1, r2) = scatter_rng[fastrand::usize(0..scatter_rng.len())];
        let (color, out_dir) = if r1 < reflection_prob {
            (
                Vec3::one() * 0.9,
                ray.dir.reflected(hit.normal) + (self.roughness * random_in_unit_sphere()),
            )
        } else {
            let direction = random_in_hemisphere(hit.normal, r1, r2);
            (self.albedo, direction)
        };
        Scatter::new(color, Ray::new(hit.point, out_dir))
    }
}

#[derive(Copy, Clone)]
pub struct Metal {
    pub albedo: Vec3,
    pub roughness: f32,
}

impl Metal {
    pub fn scatter(self, ray: Ray, hit: HitRecord, _: &[(f32, f32)]) -> Scatter {
        let out_dir = ray.dir.reflected(hit.normal) + (self.roughness * random_in_unit_sphere());
        Scatter::new(
            {
                let cosine = ((-ray.dir).dot(hit.normal)).min(1.0);
                (self.albedo + (Vec3::one() - self.albedo) * (1.0 - cosine).powi(5))
                    .clamped(Vec3::zero(), Vec3::one())
            },
            Ray::new(hit.point, out_dir),
        )
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
    pub fn scatter(self, ray: Ray, hit: HitRecord, _: &[(f32, f32)]) -> Scatter {
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
                Vec3::one() * 0.99,
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
    Glossy(Glossy),
}

impl Material {
    pub fn lambertian(albedo: (f32, f32, f32)) -> Material {
        Material::Lambertian(Lambertian {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
        })
    }

    pub fn glossy(albedo: (f32, f32, f32), roughness: f32, reflectance: f32) -> Material {
        Material::Glossy(Glossy {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            roughness,
            reflectance,
        })
    }

    pub fn metal(albedo: (f32, f32, f32), roughness: f32) -> Material {
        Material::Metal(Metal {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            roughness,
        })
    }

    pub fn dielectric(albedo: (f32, f32, f32), refractive_index: f32) -> Material {
        Material::Dielectric(Dielectric {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            refractive_index,
        })
    }

    pub fn scatter(self, ray: Ray, hit: HitRecord, scatter_rng: &[(f32, f32)]) -> Scatter {
        match hit.material.as_ref() {
            Material::Dielectric(d) => d.scatter(ray, hit, scatter_rng),
            Material::Lambertian(l) => l.scatter(ray, hit, scatter_rng),
            Material::Metal(m) => m.scatter(ray, hit, scatter_rng),
            Material::Glossy(g) => g.scatter(ray, hit, scatter_rng),
        }
    }
}
